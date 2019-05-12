use std::{
    path::PathBuf,
    fs::File,
    io::{self, Write, BufWriter},
};

use log::info;
use failure::Fail;

use crate::{
    imfs::{Imfs, FsError},
    project::{Project, ProjectLoadFuzzyError},
    rbx_session::construct_oneoff_tree,
    rbx_snapshot::SnapshotError,
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
}

#[derive(Debug, Fail)]
pub enum BuildError {
    #[fail(display = "Could not detect what kind of file to create")]
    UnknownOutputKind,

    #[fail(display = "Project load error: {}", _0)]
    ProjectLoadError(#[fail(cause)] ProjectLoadFuzzyError),

    #[fail(display = "IO error: {}", _0)]
    IoError(#[fail(cause)] io::Error),

    #[fail(display = "XML model file error")]
    XmlModelEncodeError(rbx_xml::EncodeError),

    #[fail(display = "Binary model file error")]
    BinaryModelEncodeError(rbx_binary::EncodeError),

    #[fail(display = "{}", _0)]
    FsError(#[fail(cause)] FsError),

    #[fail(display = "{}", _0)]
    SnapshotError(#[fail(cause)] SnapshotError),
}

impl_from!(BuildError {
    ProjectLoadFuzzyError => ProjectLoadError,
    io::Error => IoError,
    rbx_xml::EncodeError => XmlModelEncodeError,
    rbx_binary::EncodeError => BinaryModelEncodeError,
    FsError => FsError,
    SnapshotError => SnapshotError,
});

pub fn build(options: &BuildOptions) -> Result<(), BuildError> {
    let output_kind = options.output_kind
        .or_else(|| detect_output_kind(options))
        .ok_or(BuildError::UnknownOutputKind)?;

    info!("Hoping to generate file of type {:?}", output_kind);

    info!("Looking for project at {}", options.fuzzy_project_path.display());

    let project = Project::load_fuzzy(&options.fuzzy_project_path)?;
    project.check_compatibility();

    info!("Found project at {}", project.file_location.display());
    info!("Using project {:#?}", project);

    let mut imfs = Imfs::new();
    imfs.add_roots_from_project(&project)?;
    let tree = construct_oneoff_tree(&project, &imfs)?;
    let mut file = BufWriter::new(File::create(&options.output_file)?);

    match output_kind {
        OutputKind::Rbxmx => {
            // Model files include the root instance of the tree and all its
            // descendants.

            let root_id = tree.get_root_id();
            rbx_xml::to_writer_default(&mut file, &tree, &[root_id])?;
        },
        OutputKind::Rbxlx => {
            // Place files don't contain an entry for the DataModel, but our
            // RbxTree representation does.

            let root_id = tree.get_root_id();
            let top_level_ids = tree.get_instance(root_id).unwrap().get_children_ids();
            rbx_xml::to_writer_default(&mut file, &tree, top_level_ids)?;
        },
        OutputKind::Rbxm => {
            let root_id = tree.get_root_id();
            rbx_binary::encode(&tree, &[root_id], &mut file)?;
        },
        OutputKind::Rbxl => {
            let root_id = tree.get_root_id();
            let top_level_ids = tree.get_instance(root_id).unwrap().get_children_ids();
            rbx_binary::encode(&tree, top_level_ids, &mut file)?;
        },
    }

    file.flush()?;

    Ok(())
}