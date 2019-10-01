use std::collections::HashMap;

use insta::assert_yaml_snapshot;
use maplit::hashmap;
use rbx_dom_weak::{RbxId, RbxInstanceProperties, RbxValue};
use serde::Serialize;

use rojo_insta_ext::RedactionMap;

use crate::snapshot::{
    apply_patch_set, InstanceMetadata, InstancePropertiesWithMeta, PatchSet, PatchUpdate, RojoTree,
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

    let tree_view = view_tree(&tree);
    let tree_value = redactions.redacted_yaml(tree_view);
    assert_yaml_snapshot!(tree_value);

    let applied_patch_value = redactions.redacted_yaml(applied_patch_set);
    assert_yaml_snapshot!(applied_patch_value);
}

#[test]
fn add_property() {
    let mut redactions = RedactionMap::new();

    let mut tree = empty_tree();
    redactions.intern(tree.get_root_id());

    let patch_set = PatchSet {
        updated_instances: vec![PatchUpdate {
            id: tree.get_root_id(),
            changed_name: None,
            changed_class_name: None,
            changed_properties: hashmap! {
                "Foo".to_owned() => Some(RbxValue::String {
                    value: "Value of Foo".to_owned(),
                }),
            },
            changed_metadata: None,
        }],
        ..Default::default()
    };

    let applied_patch_set = apply_patch_set(&mut tree, patch_set);

    let tree_view = view_tree(&tree);
    let tree_value = redactions.redacted_yaml(tree_view);
    assert_yaml_snapshot!(tree_value);

    let applied_patch_value = redactions.redacted_yaml(applied_patch_set);
    assert_yaml_snapshot!(applied_patch_value);
}

#[test]
fn remove_property() {
    let mut redactions = RedactionMap::new();

    let mut tree = empty_tree();
    redactions.intern(tree.get_root_id());

    {
        let root_id = tree.get_root_id();
        let mut root_instance = tree.get_instance_mut(root_id).unwrap();

        root_instance.properties_mut().insert(
            "Foo".to_owned(),
            RbxValue::String {
                value: "Should be removed".to_owned(),
            },
        );
    }

    let tree_view = view_tree(&tree);
    let tree_value = redactions.redacted_yaml(tree_view);
    assert_yaml_snapshot!("remove_property_initial", tree_value);

    let patch_set = PatchSet {
        updated_instances: vec![PatchUpdate {
            id: tree.get_root_id(),
            changed_name: None,
            changed_class_name: None,
            changed_properties: hashmap! {
                "Foo".to_owned() => None,
            },
            changed_metadata: None,
        }],
        ..Default::default()
    };

    let applied_patch_set = apply_patch_set(&mut tree, patch_set);

    let tree_view = view_tree(&tree);
    let tree_value = redactions.redacted_yaml(tree_view);
    assert_yaml_snapshot!("remove_property_after_patch", tree_value);

    let applied_patch_value = redactions.redacted_yaml(applied_patch_set);
    assert_yaml_snapshot!("remove_property_appied_patch", applied_patch_value);
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

/// Copy of data from RojoTree in the right shape to have useful snapshots.
#[derive(Debug, Serialize)]
struct InstanceView {
    id: RbxId,
    name: String,
    class_name: String,
    properties: HashMap<String, RbxValue>,
    metadata: InstanceMetadata,
    children: Vec<InstanceView>,
}

fn view_tree(tree: &RojoTree) -> InstanceView {
    view_instance(tree, tree.get_root_id())
}

fn view_instance(tree: &RojoTree, id: RbxId) -> InstanceView {
    let instance = tree.get_instance(id).unwrap();

    InstanceView {
        id: instance.id(),
        name: instance.name().to_owned(),
        class_name: instance.class_name().to_owned(),
        properties: instance.properties().clone(),
        metadata: instance.metadata().clone(),
        children: instance
            .children()
            .iter()
            .copied()
            .map(|id| view_instance(tree, id))
            .collect(),
    }
}
