use std::{
    collections::HashMap,
    path::PathBuf,
    fs::File,
    io::{self, Write, BufWriter},
};

use rbx_dom_weak::{RbxTree, RbxInstanceProperties, RbxValue};
use log::info;
use failure::Fail;

use crate::{
    imfs::{Imfs, FsError},
    project::{Project, ProjectLoadFuzzyError},
    rbx_session::construct_oneoff_tree,
    rbx_snapshot::SnapshotError,
    commands::serve::DEFAULT_PORT,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputKind {
    XmlModel,
    XmlPlace,
    BinaryModel,
    BinaryPlace,
}

fn detect_output_kind(options: &BuildOptions) -> Option<OutputKind> {
    let extension = options.output_file.extension()?.to_str()?;

    match extension {
        "rbxlx" => Some(OutputKind::XmlPlace),
        "rbxmx" => Some(OutputKind::XmlModel),
        "rbxl" => Some(OutputKind::BinaryPlace),
        "rbxm" => Some(OutputKind::BinaryModel),
        _ => None,
    }
}

#[derive(Debug)]
pub struct BuildOptions {
    pub fuzzy_project_path: PathBuf,
    pub output_file: PathBuf,
    pub output_kind: Option<OutputKind>,
    pub plugin_autostart: bool,
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

    #[fail(display = "plugin_autostart cannot be enabled when building models")]
    PluginAutostartOnModelError,
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

    let mut tree = construct_oneoff_tree(&project, &imfs)?;

    if options.plugin_autostart {
        match output_kind {
            OutputKind::BinaryPlace | OutputKind::XmlPlace => {
                let port = project.serve_port.unwrap_or(DEFAULT_PORT);
                inject_autostart_marker(&mut tree, port);
            }
            _ => return Err(BuildError::PluginAutostartOnModelError)
        }
    }

    let ids_to_encode = match output_kind {
        OutputKind::XmlPlace | OutputKind::BinaryPlace => {
            // Place files don't include their root instance.

            let root_id = tree.get_root_id();
            let root = tree.get_instance(root_id).unwrap();

            root.get_children_ids().to_vec()
        }
        OutputKind::XmlModel | OutputKind::BinaryModel =>  {
            // Model files include the root instance and all its descendants.

            vec![tree.get_root_id()]
        }
    };

    let mut file = BufWriter::new(File::create(&options.output_file)?);

    match output_kind {
        OutputKind::BinaryPlace | OutputKind::BinaryModel => {
            rbx_binary::encode(&tree, &ids_to_encode, &mut file)?;
        }
        OutputKind::XmlPlace | OutputKind::XmlModel => {
            rbx_xml::encode(&tree, &ids_to_encode, &mut file)?;
        }
    }

    file.flush()?;

    Ok(())
}

fn inject_autostart_marker(tree: &mut RbxTree, port: u16) {
    let root_id = tree.get_root_id();

    let mut properties = HashMap::new();
    properties.insert(String::from("Value"), RbxValue::Int64 { value: port as i64 });

    let marker = RbxInstanceProperties {
        class_name: String::from("IntValue"),
        name: String::from("ROJO_AUTOSTART_PORT"),
        properties,
    };

    tree.insert_instance(marker, root_id);
}