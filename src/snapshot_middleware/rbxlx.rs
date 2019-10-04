use std::borrow::Cow;

use rbx_dom_weak::{RbxId, RbxTree};

use crate::{
    imfs::{Imfs, ImfsEntry, ImfsFetcher},
    snapshot::InstanceSnapshot,
};

use super::middleware::{SnapshotFileResult, SnapshotInstanceResult, SnapshotMiddleware};

pub struct SnapshotRbxlx;

impl SnapshotMiddleware for SnapshotRbxlx {
    fn from_imfs<F: ImfsFetcher>(
        imfs: &mut Imfs<F>,
        entry: &ImfsEntry,
    ) -> SnapshotInstanceResult<'static> {
        if entry.is_directory() {
            return Ok(None);
        }

        let file_name = entry.path().file_name().unwrap().to_string_lossy();

        if !file_name.ends_with(".rbxlx") {
            return Ok(None);
        }

        let instance_name = entry
            .path()
            .file_stem()
            .expect("Could not extract file stem")
            .to_string_lossy()
            .to_string();

        let options = rbx_xml::DecodeOptions::new()
            .property_behavior(rbx_xml::DecodePropertyBehavior::ReadUnknown);

        let temp_tree = rbx_xml::from_reader(entry.contents(imfs)?, options)
            .expect("TODO: Handle rbx_xml errors");

        let root_id = temp_tree.get_root_id();

        let mut snapshot = InstanceSnapshot::from_tree(&temp_tree, root_id);
        snapshot.name = Cow::Owned(instance_name);
        snapshot.metadata.contributing_paths = vec![entry.path().to_path_buf()];

        Ok(Some(snapshot))
    }

    fn from_instance(_tree: &RbxTree, _id: RbxId) -> SnapshotFileResult {
        None
    }
}
