use std::path::Path;

use memofs::Vfs;

use crate::snapshot::{InstanceContext, InstanceMetadata, InstanceSnapshot};

use super::{
    middleware::{SnapshotInstanceResult, SnapshotMiddleware},
    util::match_file_name,
};

pub struct SnapshotRbxlx;

impl SnapshotMiddleware for SnapshotRbxlx {
    fn from_vfs(context: &InstanceContext, vfs: &Vfs, path: &Path) -> SnapshotInstanceResult {
        let meta = vfs.metadata(path)?;

        if meta.is_dir() {
            return Ok(None);
        }

        let instance_name = match match_file_name(path, ".rbxlx") {
            Some(name) => name,
            None => return Ok(None),
        };

        let options = rbx_xml::DecodeOptions::new()
            .property_behavior(rbx_xml::DecodePropertyBehavior::ReadUnknown);

        let temp_tree = rbx_xml::from_reader(vfs.read(path)?.as_slice(), options)
            .expect("TODO: Handle rbx_xml errors");

        let root_id = temp_tree.get_root_id();

        let snapshot = InstanceSnapshot::from_tree(&temp_tree, root_id)
            .name(instance_name)
            .metadata(
                InstanceMetadata::new()
                    .instigating_source(path)
                    .relevant_paths(vec![path.to_path_buf()])
                    .context(context),
            );

        Ok(Some(snapshot))
    }
}
