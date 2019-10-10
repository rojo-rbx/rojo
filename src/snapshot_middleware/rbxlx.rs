use std::borrow::Cow;

use rbx_dom_weak::{RbxId, RbxTree};

use crate::{
    imfs::{Imfs, ImfsEntry, ImfsFetcher},
    snapshot::InstanceSnapshot,
};

use super::{
    middleware::{SnapshotFileResult, SnapshotInstanceResult, SnapshotMiddleware},
    util::match_file_name,
};

pub struct SnapshotRbxlx;

impl SnapshotMiddleware for SnapshotRbxlx {
    fn from_imfs<F: ImfsFetcher>(
        imfs: &mut Imfs<F>,
        entry: &ImfsEntry,
    ) -> SnapshotInstanceResult<'static> {
        if entry.is_directory() {
            return Ok(None);
        }

        let instance_name = match match_file_name(entry.path(), ".rbxlx") {
            Some(name) => name,
            None => return Ok(None),
        };

        let options = rbx_xml::DecodeOptions::new()
            .property_behavior(rbx_xml::DecodePropertyBehavior::ReadUnknown);

        let temp_tree = rbx_xml::from_reader(entry.contents(imfs)?, options)
            .expect("TODO: Handle rbx_xml errors");

        let root_id = temp_tree.get_root_id();

        let mut snapshot = InstanceSnapshot::from_tree(&temp_tree, root_id);
        snapshot.name = Cow::Owned(instance_name.to_owned());
        snapshot.metadata.instigating_source = Some(entry.path().to_path_buf().into());
        snapshot.metadata.relevant_paths = vec![entry.path().to_path_buf()];

        Ok(Some(snapshot))
    }

    fn from_instance(_tree: &RbxTree, _id: RbxId) -> SnapshotFileResult {
        None
    }
}
