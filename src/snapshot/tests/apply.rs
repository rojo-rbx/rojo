use rbx_dom_weak::RbxInstanceProperties;

use crate::snapshot::{
    apply_patch_set, InstancePropertiesWithMeta, PatchSet, PatchUpdate, RojoTree,
};

#[test]
fn reify_folder() {
    let mut tree = empty_tree();

    let patch_set = PatchSet {
        updated_instances: vec![PatchUpdate {
            id: tree.get_root_id(),
            changed_name: Some("Hello, world!".to_owned()),
            changed_class_name: Some("Folder".to_owned()),
            changed_properties: Default::default(),
            changed_metadata: None,
        }],
        ..Default::default()
    };

    let _applied_patch_set = apply_patch_set(&mut tree, patch_set);

    // TODO: Make assertions about tree using snapshots
    // TODO: make assertions about applied patch set using snapshots
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
