use std::{
    path::PathBuf,
    fs::File,
    io,
};

use failure::Fail;

use crate::{
    rbx_session::construct_oneoff_tree,
    project::{Project, ProjectLoadFuzzyError},
    imfs::Imfs,
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
    BinaryModelEncodeError(rbx_binary::EncodeError)
}

impl From<ProjectLoadFuzzyError> for BuildError {
    fn from(error: ProjectLoadFuzzyError) -> BuildError {
        BuildError::ProjectLoadError(error)
    }
}

impl From<io::Error> for BuildError {
    fn from(error: io::Error) -> BuildError {
        BuildError::IoError(error)
    }
}

impl From<rbx_xml::EncodeError> for BuildError {
    fn from(error: rbx_xml::EncodeError) -> BuildError {
        BuildError::XmlModelEncodeError(error)
    }
}

impl From<rbx_binary::EncodeError> for BuildError {
    fn from(error: rbx_binary::EncodeError) -> BuildError {
        BuildError::BinaryModelEncodeError(error)
    }
}

pub fn build(options: &BuildOptions) -> Result<(), BuildError> {
    let output_kind = options.output_kind
        .or_else(|| detect_output_kind(options))
        .ok_or(BuildError::UnknownOutputKind)?;

    info!("Hoping to generate file of type {:?}", output_kind);

    info!("Looking for project at {}", options.fuzzy_project_path.display());

    let project = Project::load_fuzzy(&options.fuzzy_project_path)?;

    info!("Found project at {}", project.file_location.display());
    info!("Using project {:#?}", project);

    let mut imfs = Imfs::new();
    imfs.add_roots_from_project(&project)?;
    let tree = construct_oneoff_tree(&project, &imfs);
    let mut file = File::create(&options.output_file)?;

    match output_kind {
        OutputKind::Rbxmx => {
            // Model files include the root instance of the tree and all its
            // descendants.

            let root_id = tree.get_root_id();
            rbx_xml::encode(&tree, &[root_id], &mut file)?;
        },
        OutputKind::Rbxlx => {
            // Place files don't contain an entry for the DataModel, but our
            // RbxTree representation does.

            let root_id = tree.get_root_id();
            let top_level_ids = tree.get_instance(root_id).unwrap().get_children_ids();
            rbx_xml::encode(&tree, top_level_ids, &mut file)?;
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

    Ok(())
}