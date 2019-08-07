use std::{
    collections::HashMap,
    fs::File,
    io::{self, Write, BufWriter},
    path::PathBuf,
};

use rbx_dom_weak::{RbxTree, RbxInstanceProperties};
use log::info;
use failure::Fail;

use crate::{
    imfs::new::{Imfs, RealFetcher, FsError},
    snapshot::{PatchSet, apply_patch, compute_patch_set},
    snapshot_middleware::snapshot_from_imfs,
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

    #[fail(display = "IO error: {}", _0)]
    IoError(#[fail(cause)] io::Error),

    #[fail(display = "XML model file error")]
    XmlModelEncodeError(rbx_xml::EncodeError),

    #[fail(display = "Binary model file error")]
    BinaryModelEncodeError(rbx_binary::EncodeError),

    #[fail(display = "{}", _0)]
    FsError(#[fail(cause)] FsError),
}

impl_from!(BuildError {
    io::Error => IoError,
    rbx_xml::EncodeError => XmlModelEncodeError,
    rbx_binary::EncodeError => BinaryModelEncodeError,
    FsError => FsError,
});

fn xml_encode_config() -> rbx_xml::EncodeOptions {
    rbx_xml::EncodeOptions::new()
        .property_behavior(rbx_xml::EncodePropertyBehavior::WriteUnknown)
}

pub fn build(options: &BuildOptions) -> Result<(), BuildError> {
    let output_kind = options.output_kind
        .or_else(|| detect_output_kind(options))
        .ok_or(BuildError::UnknownOutputKind)?;

    info!("Hoping to generate file of type {:?}", output_kind);

    let mut tree = RbxTree::new(RbxInstanceProperties {
        name: "ROOT".to_owned(),
        class_name: "Folder".to_owned(),
        properties: HashMap::new(),
    });
    let root_id = tree.get_root_id();

    let mut imfs = Imfs::new(RealFetcher);
    let entry = imfs.get(&options.fuzzy_project_path)
        .expect("could not get project path");

    let snapshot = snapshot_from_imfs(&mut imfs, &entry)
        .expect("snapshot failed")
        .expect("snapshot did not return an instance");

    let mut patch_set = PatchSet::new();
    compute_patch_set(&snapshot, &tree, root_id, &mut patch_set);
    apply_patch(&mut tree, &patch_set);

    let mut file = BufWriter::new(File::create(&options.output_file)?);

    match output_kind {
        OutputKind::Rbxmx => {
            // Model files include the root instance of the tree and all its
            // descendants.

            rbx_xml::to_writer(&mut file, &tree, &[root_id], xml_encode_config())?;
        },
        OutputKind::Rbxlx => {
            // Place files don't contain an entry for the DataModel, but our
            // RbxTree representation does.

            let top_level_ids = tree.get_instance(root_id).unwrap().get_children_ids();
            rbx_xml::to_writer(&mut file, &tree, top_level_ids, xml_encode_config())?;
        },
        OutputKind::Rbxm => {
            rbx_binary::encode(&tree, &[root_id], &mut file)?;
        },
        OutputKind::Rbxl => {
            log::warn!("Support for building binary places (rbxl) is still experimental.");
            log::warn!("Using the XML place format (rbxlx) is recommended instead.");
            log::warn!("For more info, see https://github.com/LPGhatguy/rojo/issues/180");

            let top_level_ids = tree.get_instance(root_id).unwrap().get_children_ids();
            rbx_binary::encode(&tree, top_level_ids, &mut file)?;
        },
    }

    file.flush()?;

    Ok(())
}