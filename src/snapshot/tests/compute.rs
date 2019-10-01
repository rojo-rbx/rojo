use std::borrow::Cow;

use insta::assert_yaml_snapshot;
use maplit::hashmap;
use rbx_dom_weak::{RbxInstanceProperties, RbxValue};

use rojo_insta_ext::RedactionMap;

use crate::snapshot::{compute_patch_set, InstancePropertiesWithMeta, InstanceSnapshot, RojoTree};

#[test]
fn set_name_and_class_name() {
    let mut redactions = RedactionMap::new();

    let tree = empty_tree();
    redactions.intern(tree.get_root_id());

    let snapshot = InstanceSnapshot {
        snapshot_id: None,
        metadata: Default::default(),
        name: Cow::Borrowed("Some Folder"),
        class_name: Cow::Borrowed("Folder"),
        properties: Default::default(),
        children: Vec::new(),
    };

    let patch_set = compute_patch_set(&snapshot, &tree, tree.get_root_id());
    let patch_value = redactions.redacted_yaml(patch_set);

    assert_yaml_snapshot!(patch_value);
}

#[test]
fn set_property() {
    let mut redactions = RedactionMap::new();

    let tree = empty_tree();
    redactions.intern(tree.get_root_id());

    let snapshot = InstanceSnapshot {
        snapshot_id: None,
        metadata: Default::default(),
        name: Cow::Borrowed("ROOT"),
        class_name: Cow::Borrowed("ROOT"),
        properties: hashmap! {
            "PropertyName".to_owned() => RbxValue::String {
                value: "Hello, world!".to_owned(),
            },
        },
        children: Vec::new(),
    };

    let patch_set = compute_patch_set(&snapshot, &tree, tree.get_root_id());
    let patch_value = redactions.redacted_yaml(patch_set);

    assert_yaml_snapshot!(patch_value);
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
                value: "This should be removed by the patch.".to_owned(),
            },
        );
    }

    let snapshot = InstanceSnapshot {
        snapshot_id: None,
        metadata: Default::default(),
        name: Cow::Borrowed("ROOT"),
        class_name: Cow::Borrowed("ROOT"),
        properties: Default::default(),
        children: Vec::new(),
    };

    let patch_set = compute_patch_set(&snapshot, &tree, tree.get_root_id());
    let patch_value = redactions.redacted_yaml(patch_set);

    assert_yaml_snapshot!(patch_value);
}

#[test]
fn add_child() {
    let mut redactions = RedactionMap::new();

    let tree = empty_tree();
    redactions.intern(tree.get_root_id());

    let snapshot = InstanceSnapshot {
        snapshot_id: None,
        metadata: Default::default(),
        name: Cow::Borrowed("ROOT"),
        class_name: Cow::Borrowed("ROOT"),
        properties: Default::default(),
        children: vec![InstanceSnapshot {
            snapshot_id: None,
            metadata: Default::default(),
            name: Cow::Borrowed("New"),
            class_name: Cow::Borrowed("Folder"),
            properties: Default::default(),
            children: Vec::new(),
        }],
    };

    let patch_set = compute_patch_set(&snapshot, &tree, tree.get_root_id());
    let patch_value = redactions.redacted_yaml(patch_set);

    assert_yaml_snapshot!(patch_value);
}

#[test]
fn remove_child() {
    let mut redactions = RedactionMap::new();

    let mut tree = empty_tree();
    redactions.intern(tree.get_root_id());

    {
        let root_id = tree.get_root_id();
        let new_id = tree.insert_instance(
            InstancePropertiesWithMeta {
                properties: RbxInstanceProperties {
                    name: "Should not appear in snapshot".to_owned(),
                    class_name: "Folder".to_owned(),
                    properties: Default::default(),
                },
                metadata: Default::default(),
            },
            root_id,
        );

        redactions.intern(new_id);
    }

    let snapshot = InstanceSnapshot {
        snapshot_id: None,
        metadata: Default::default(),
        name: Cow::Borrowed("ROOT"),
        class_name: Cow::Borrowed("ROOT"),
        properties: Default::default(),
        children: Vec::new(),
    };

    let patch_set = compute_patch_set(&snapshot, &tree, tree.get_root_id());
    let patch_value = redactions.redacted_yaml(patch_set);

    assert_yaml_snapshot!(patch_value);
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
