use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
};

use failure::Fail;
use rbx_dom_weak::{RbxId, RbxTree};

use crate::{
    common_setup,
    project::ProjectLoadError,
    vfs::{FsError, RealFetcher, Vfs, WatchMode},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputKind {
    Rbxmx,
    Rbxlx,
    Rbxm,
    Rbxl,
}

fn detect_output_kind(options: &BuildOptions) -> Option<OutputKind> {
    let extension = options.output_file.extension()?.to_str()?;

    match extension {
        "rbxlx" => Some(OutputKind::Rbxlx),
        "rbxmx" => Some(OutputKind::Rbxmx),
        "rbxl" => Some(OutputKind::Rbxl),
        "rbxm" => Some(OutputKind::Rbxm),
        _ => None,
    }
}

#[derive(Debug)]
pub struct BuildOptions {
    pub fuzzy_project_path: PathBuf,
    pub output_file: PathBuf,
    pub output_kind: Option<OutputKind>,
    pub output_sourcemap: bool,
}

#[derive(Debug, Fail)]
pub enum BuildError {
    #[fail(display = "Could not detect what kind of file to create")]
    UnknownOutputKind,

    #[fail(display = "IO error: {}", _0)]
    IoError(#[fail(cause)] io::Error),

    #[fail(display = "{}", _0)]
    JsonEncodeError(#[fail(cause)] serde_json::Error),

    #[fail(display = "XML model error: {}", _0)]
    XmlModelEncodeError(#[fail(cause)] rbx_xml::EncodeError),

    #[fail(display = "Binary model error: {:?}", _0)]
    BinaryModelEncodeError(rbx_binary::EncodeError),

    #[fail(display = "{}", _0)]
    ProjectLoadError(#[fail(cause)] ProjectLoadError),

    #[fail(display = "{}", _0)]
    FsError(#[fail(cause)] FsError),
}

impl_from!(BuildError {
    io::Error => IoError,
    serde_json::Error => JsonEncodeError,
    rbx_xml::EncodeError => XmlModelEncodeError,
    rbx_binary::EncodeError => BinaryModelEncodeError,
    ProjectLoadError => ProjectLoadError,
    FsError => FsError,
});

fn xml_encode_config() -> rbx_xml::EncodeOptions {
    rbx_xml::EncodeOptions::new().property_behavior(rbx_xml::EncodePropertyBehavior::WriteUnknown)
}

pub fn build(options: &BuildOptions) -> Result<(), BuildError> {
    let output_kind = options
        .output_kind
        .or_else(|| detect_output_kind(options))
        .ok_or(BuildError::UnknownOutputKind)?;

    log::debug!("Hoping to generate file of type {:?}", output_kind);

    log::trace!("Constructing in-memory filesystem");
    let vfs = Vfs::new(RealFetcher::new(WatchMode::Disabled));

    let (_maybe_project, tree) = common_setup::start(&options.fuzzy_project_path, &vfs);
    let root_id = tree.get_root_id();

    log::trace!("Opening output file for write");
    let mut file = BufWriter::new(File::create(&options.output_file)?);

    match output_kind {
        OutputKind::Rbxmx => {
            // Model files include the root instance of the tree and all its
            // descendants.

            rbx_xml::to_writer(&mut file, tree.inner(), &[root_id], xml_encode_config())?;
        }
        OutputKind::Rbxlx => {
            // Place files don't contain an entry for the DataModel, but our
            // RbxTree representation does.

            let root_instance = tree.get_instance(root_id).unwrap();
            let top_level_ids = root_instance.children();

            rbx_xml::to_writer(&mut file, tree.inner(), top_level_ids, xml_encode_config())?;
        }
        OutputKind::Rbxm => {
            rbx_binary::encode(tree.inner(), &[root_id], &mut file)?;
        }
        OutputKind::Rbxl => {
            log::warn!("Support for building binary places (rbxl) is still experimental.");
            log::warn!("Using the XML place format (rbxlx) is recommended instead.");
            log::warn!("For more info, see https://github.com/LPGhatguy/rojo/issues/180");

            let root_instance = tree.get_instance(root_id).unwrap();
            let top_level_ids = root_instance.children();

            rbx_binary::encode(tree.inner(), top_level_ids, &mut file)?;
        }
    }

    file.flush()?;

    if options.output_sourcemap {
        log::trace!("Computing sourcemap");

        let mut map_data: HashMap<String, Vec<&Path>> = HashMap::new();

        for (path, ids) in tree.known_paths() {
            for &id in ids {
                let name = get_full_name(tree.inner(), id);
                let relevant_paths = map_data.entry(name).or_insert(Vec::new());
                relevant_paths.push(path);
            }
        }

        let map_path = {
            // This should not panic because we make assertions about the file
            // name earlier in this function.
            //
            // It may panic if the file name is not valid UTF-8. We need to do a
            // conversion like this since the representation of OsStr is
            // platform-dependent.
            let existing_name = options.output_file.file_name().unwrap().to_str().unwrap();

            let mut new_name = existing_name.to_owned();
            new_name.push_str(".map");

            options.output_file.with_file_name(new_name)
        };

        log::trace!("Writing sourcemap to {}", map_path.display());

        let mut map_file = BufWriter::new(File::create(map_path)?);
        serde_json::to_writer_pretty(&mut map_file, &map_data)?;
        map_file.flush()?;
    }

    Ok(())
}

/// Returns a a slash-delimited full name for the given instance by traversing
/// upwards in the tree.
///
/// If the tree contains only uniquely named siblings, this path can be used to
/// identify the given instance.
fn get_full_name(tree: &RbxTree, id: RbxId) -> String {
    let mut components_reversed = Vec::new();
    let mut current_id = Some(id);

    while let Some(id) = current_id {
        let instance = tree.get_instance(id).unwrap();
        components_reversed.push(instance.name.as_str());
        current_id = instance.get_parent_id();
    }

    let mut name = String::new();
    for component in components_reversed.iter().rev() {
        name.push_str(component);
        name.push('/');
    }
    name.pop();

    name
}
