use std::borrow::Cow;

use rbx_dom_weak::RbxInstanceProperties;

use crate::snapshot::{compute_patch_set, InstancePropertiesWithMeta, InstanceSnapshot, RojoTree};

#[test]
fn reify_folder() {
    let tree = empty_tree();

    let folder = InstanceSnapshot {
        snapshot_id: None,
        metadata: Default::default(),
        name: Cow::Borrowed("Some Folder"),
        class_name: Cow::Borrowed("Folder"),
        properties: Default::default(),
        children: Vec::new(),
    };

    let _patch_set = compute_patch_set(&folder, &tree, tree.get_root_id());

    // TODO: Make assertions about patch set using snapshots. This needs patches
    // to be serializable and also to have ID redactions more readily available.
}

fn empty_tree() -> RojoTree {
    RojoTree::new(InstancePropertiesWithMeta {
        properties: RbxInstanceProperties {
            name: "ROOT".to_owned(),
            class_name: "ROOT".to_owned(),
            properties: Default::default(),
        },
        metadata: Default::default(),
    })
}
