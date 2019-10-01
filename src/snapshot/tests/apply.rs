use insta::assert_yaml_snapshot;
use rbx_dom_weak::RbxInstanceProperties;

use rojo_insta_ext::RedactionMap;

use crate::snapshot::{
    apply_patch_set, InstancePropertiesWithMeta, PatchSet, PatchUpdate, RojoTree,
};

#[test]
fn set_name_and_class_name() {
    let mut redactions = RedactionMap::new();

    let mut tree = empty_tree();
    redactions.intern(tree.get_root_id());

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

    let applied_patch_set = apply_patch_set(&mut tree, patch_set);

    // TODO: Snapshot tree, requires RojoTree: Serialize (but not Deserialize!)

    let applied_patch_value = redactions.redacted_yaml(applied_patch_set);
    assert_yaml_snapshot!(applied_patch_value);
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
