use std::{borrow::Cow, collections::HashMap};

use rbx_dom_weak::{RbxId, RbxInstanceProperties, RbxTree};

use crate::{
    imfs::{Imfs, ImfsEntry, ImfsFetcher},
    snapshot::InstanceSnapshot,
};

use super::middleware::{SnapshotFileResult, SnapshotInstanceResult, SnapshotMiddleware};

pub struct SnapshotRbxm;

impl SnapshotMiddleware for SnapshotRbxm {
    fn from_imfs<F: ImfsFetcher>(
        imfs: &mut Imfs<F>,
        entry: &ImfsEntry,
    ) -> SnapshotInstanceResult<'static> {
        if entry.is_directory() {
            return Ok(None);
        }

        let file_name = entry.path().file_name().unwrap().to_string_lossy();

        if !file_name.ends_with(".rbxm") {
            return Ok(None);
        }

        let instance_name = entry
            .path()
            .file_stem()
            .expect("Could not extract file stem")
            .to_string_lossy()
            .to_string();

        let mut temp_tree = RbxTree::new(RbxInstanceProperties {
            name: "DataModel".to_owned(),
            class_name: "DataModel".to_owned(),
            properties: HashMap::new(),
        });

        let root_id = temp_tree.get_root_id();
        rbx_binary::decode(&mut temp_tree, root_id, entry.contents(imfs)?)
            .expect("TODO: Handle rbx_binary errors");

        let root_instance = temp_tree.get_instance(root_id).unwrap();
        let children = root_instance.get_children_ids();

        if children.len() == 1 {
            let mut snapshot = InstanceSnapshot::from_tree(&temp_tree, children[0]);
            snapshot.name = Cow::Owned(instance_name);
            snapshot.metadata.instigating_source = Some(entry.path().to_path_buf().into());
            snapshot.metadata.relevant_paths = vec![entry.path().to_path_buf()];

            Ok(Some(snapshot))
        } else {
            panic!("Rojo doesn't have support for model files with zero or more than one top-level instances yet.");
        }
    }

    fn from_instance(_tree: &RbxTree, _id: RbxId) -> SnapshotFileResult {
        unimplemented!("Snapshotting models");
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::imfs::{ImfsDebug, ImfsSnapshot, NoopFetcher};

    #[test]
    fn model_from_imfs() {
        let mut imfs = Imfs::new(NoopFetcher);
        let file = ImfsSnapshot::file(include_bytes!("../../assets/test-folder.rbxm").to_vec());

        imfs.debug_load_snapshot("/foo.rbxm", file);

        let entry = imfs.get("/foo.rbxm").unwrap();
        let instance_snapshot = SnapshotRbxm::from_imfs(&mut imfs, &entry).unwrap().unwrap();

        assert_eq!(instance_snapshot.name, "foo");
        assert_eq!(instance_snapshot.class_name, "Folder");
        assert_eq!(instance_snapshot.children, Vec::new());

        // We intentionally don't assert on properties. rbx_binary does not
        // distinguish between String and BinaryString. The sample model was
        // created by Roblox Studio and has an empty BinaryString "Tags"
        // property that currently deserializes incorrectly.
        // See: https://github.com/rojo-rbx/rbx-dom/issues/49
    }
}
