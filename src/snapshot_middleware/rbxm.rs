use std::{collections::HashMap, path::Path};

use memofs::Vfs;
use rbx_dom_weak::{RbxInstanceProperties, RbxTree};

use crate::snapshot::{InstanceContext, InstanceMetadata, InstanceSnapshot};

use super::middleware::SnapshotInstanceResult;

pub fn snapshot_rbxm(
    context: &InstanceContext,
    vfs: &Vfs,
    path: &Path,
    instance_name: &str,
) -> SnapshotInstanceResult {
    let mut temp_tree = RbxTree::new(RbxInstanceProperties {
        name: "DataModel".to_owned(),
        class_name: "DataModel".to_owned(),
        properties: HashMap::new(),
    });

    let root_id = temp_tree.get_root_id();
    rbx_binary::decode(&mut temp_tree, root_id, vfs.read(path)?.as_slice())
        .expect("TODO: Handle rbx_binary errors");

    let root_instance = temp_tree.get_instance(root_id).unwrap();
    let children = root_instance.get_children_ids();

    if children.len() == 1 {
        let snapshot = InstanceSnapshot::from_tree(&temp_tree, children[0])
            .name(instance_name)
            .metadata(
                InstanceMetadata::new()
                    .instigating_source(path)
                    .relevant_paths(vec![path.to_path_buf()])
                    .context(context),
            );

        Ok(Some(snapshot))
    } else {
        panic!("Rojo doesn't have support for model files with zero or more than one top-level instances yet.");
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use memofs::{InMemoryFs, VfsSnapshot};

    #[test]
    fn model_from_vfs() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(
            "/foo.rbxm",
            VfsSnapshot::file(include_bytes!("../../assets/test-folder.rbxm").to_vec()),
        )
        .unwrap();

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_rbxm(
            &InstanceContext::default(),
            &mut vfs,
            Path::new("/foo.rbxm"),
            "foo",
        )
        .unwrap()
        .unwrap();

        assert_eq!(instance_snapshot.name, "foo");
        assert_eq!(instance_snapshot.class_name, "Folder");
        assert_eq!(instance_snapshot.children, Vec::new());

        // We intentionally don't assert on properties. rbx_binary does not
        // distinguish between String and BinaryString. The sample model was
        // created by Roblox Studio and has an empty BinaryString "Tags"
        // property that currently deserializes incorrectly.
        // See: https://github.com/Roblox/rbx-dom/issues/49
    }
}
