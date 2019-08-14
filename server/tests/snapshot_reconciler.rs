mod test_util;

use std::collections::HashMap;

use rbx_dom_weak::{RbxTree, RbxInstanceProperties};

use librojo::{
    snapshot_reconciler::{RbxSnapshotInstance, reconcile_subtree},
};

use test_util::tree::trees_equal;

#[test]
fn patch_communicativity() {
    let base_tree = RbxTree::new(RbxInstanceProperties {
        name: "DataModel".into(),
        class_name: "DataModel".into(),
        properties: HashMap::new(),
    });

    let patch_a = RbxSnapshotInstance {
        name: "DataModel".into(),
        class_name: "DataModel".into(),
        children: vec![
            RbxSnapshotInstance {
                name: "Child-A".into(),
                class_name: "Folder".into(),
                ..Default::default()
            },
        ],
        ..Default::default()
    };

    let patch_b = RbxSnapshotInstance {
        name: "DataModel".into(),
        class_name: "DataModel".into(),
        children: vec![
            RbxSnapshotInstance {
                name: "Child-B".into(),
                class_name: "Folder".into(),
                ..Default::default()
            },
        ],
        ..Default::default()
    };

    let patch_combined = RbxSnapshotInstance {
        name: "DataModel".into(),
        class_name: "DataModel".into(),
        children: vec![
            RbxSnapshotInstance {
                name: "Child-A".into(),
                class_name: "Folder".into(),
                ..Default::default()
            },
            RbxSnapshotInstance {
                name: "Child-B".into(),
                class_name: "Folder".into(),
                ..Default::default()
            },
        ],
        ..Default::default()
    };

    let root_id = base_tree.get_root_id();

    let mut tree_a = base_tree.clone();

    reconcile_subtree(
        &mut tree_a,
        root_id,
        &patch_a,
        &mut Default::default(),
        &mut Default::default(),
        &mut Default::default(),
    );

    reconcile_subtree(
        &mut tree_a,
        root_id,
        &patch_combined,
        &mut Default::default(),
        &mut Default::default(),
        &mut Default::default(),
    );

    let mut tree_b = base_tree.clone();

    reconcile_subtree(
        &mut tree_b,
        root_id,
        &patch_b,
        &mut Default::default(),
        &mut Default::default(),
        &mut Default::default(),
    );

    reconcile_subtree(
        &mut tree_b,
        root_id,
        &patch_combined,
        &mut Default::default(),
        &mut Default::default(),
        &mut Default::default(),
    );

    match trees_equal(&tree_a, &tree_b) {
        Ok(_) => {}
        Err(e) => panic!("{}", e),
    }
}