use std::collections::HashMap;

use rbx_dom_weak::{RbxInstanceProperties, RbxTree};

use crate::{
    snapshot::{InstanceContext, InstanceMetadata, InstanceSnapshot},
    vfs::{Vfs, VfsEntry, VfsFetcher},
};

use super::{
    middleware::{SnapshotInstanceResult, SnapshotMiddleware},
    util::match_file_name,
};

pub struct SnapshotRbxm;

impl SnapshotMiddleware for SnapshotRbxm {
    fn from_vfs<F: VfsFetcher>(
        _context: &InstanceContext,
        vfs: &Vfs<F>,
        entry: &VfsEntry,
    ) -> SnapshotInstanceResult {
        if entry.is_directory() {
            return Ok(None);
        }

        let instance_name = match match_file_name(entry.path(), ".rbxm") {
            Some(name) => name,
            None => return Ok(None),
        };

        let mut temp_tree = RbxTree::new(RbxInstanceProperties {
            name: "DataModel".to_owned(),
            class_name: "DataModel".to_owned(),
            properties: HashMap::new(),
        });

        let root_id = temp_tree.get_root_id();
        rbx_binary::decode(&mut temp_tree, root_id, entry.contents(vfs)?.as_slice())
            .expect("TODO: Handle rbx_binary errors");

        let root_instance = temp_tree.get_instance(root_id).unwrap();
        let children = root_instance.get_children_ids();

        if children.len() == 1 {
            let snapshot = InstanceSnapshot::from_tree(&temp_tree, children[0])
                .name(instance_name)
                .metadata(
                    InstanceMetadata::new()
                        .instigating_source(entry.path())
                        .relevant_paths(vec![entry.path().to_path_buf()]),
                );

            Ok(Some(snapshot))
        } else {
            panic!("Rojo doesn't have support for model files with zero or more than one top-level instances yet.");
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::vfs::{NoopFetcher, VfsDebug, VfsSnapshot};

    #[test]
    fn model_from_vfs() {
        let mut vfs = Vfs::new(NoopFetcher);
        let file = VfsSnapshot::file(include_bytes!("../../assets/test-folder.rbxm").to_vec());

        vfs.debug_load_snapshot("/foo.rbxm", file);

        let entry = vfs.get("/foo.rbxm").unwrap();
        let instance_snapshot =
            SnapshotRbxm::from_vfs(&InstanceContext::default(), &mut vfs, &entry)
                .unwrap()
                .unwrap();

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
