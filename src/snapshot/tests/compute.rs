use std::borrow::Cow;

use insta::assert_yaml_snapshot;

use rbx_dom_weak::{types::Ref, ustr, UstrMap};
use rojo_insta_ext::RedactionMap;

use crate::snapshot::{compute_patch_set, InstanceSnapshot, RojoTree};

#[test]
fn set_name_and_class_name() {
    let mut redactions = RedactionMap::default();

    let tree = empty_tree();
    redactions.intern(tree.get_root_id());

    let snapshot = InstanceSnapshot {
        snapshot_id: Ref::none(),
        metadata: Default::default(),
        name: Cow::Borrowed("Some Folder"),
        class_name: ustr("Folder"),
        properties: Default::default(),
        children: Vec::new(),
    };

    let patch_set = compute_patch_set(Some(snapshot), &tree, tree.get_root_id());
    let patch_value = redactions.redacted_yaml(patch_set);

    assert_yaml_snapshot!(patch_value);
}

#[test]
fn set_property() {
    let mut redactions = RedactionMap::default();

    let tree = empty_tree();
    redactions.intern(tree.get_root_id());

    let snapshot = InstanceSnapshot {
        snapshot_id: Ref::none(),
        metadata: Default::default(),
        name: Cow::Borrowed("ROOT"),
        class_name: ustr("ROOT"),
        properties: UstrMap::from_iter([(ustr("PropertyName"), "Hello, world!".into())]),
        children: Vec::new(),
    };

    let patch_set = compute_patch_set(Some(snapshot), &tree, tree.get_root_id());
    let patch_value = redactions.redacted_yaml(patch_set);

    assert_yaml_snapshot!(patch_value);
}

#[test]
fn remove_property() {
    let mut redactions = RedactionMap::default();

    let mut tree = empty_tree();
    redactions.intern(tree.get_root_id());

    {
        let root_id = tree.get_root_id();
        let mut root_instance = tree.get_instance_mut(root_id).unwrap();
        root_instance
            .properties_mut()
            .insert(ustr("Foo"), "This should be removed by the patch.".into());
    }

    let snapshot = InstanceSnapshot {
        snapshot_id: Ref::none(),
        metadata: Default::default(),
        name: Cow::Borrowed("ROOT"),
        class_name: ustr("ROOT"),
        properties: Default::default(),
        children: Vec::new(),
    };

    let patch_set = compute_patch_set(Some(snapshot), &tree, tree.get_root_id());
    let patch_value = redactions.redacted_yaml(patch_set);

    assert_yaml_snapshot!(patch_value);
}

#[test]
fn add_child() {
    let mut redactions = RedactionMap::default();

    let tree = empty_tree();
    redactions.intern(tree.get_root_id());

    let snapshot = InstanceSnapshot {
        snapshot_id: Ref::none(),
        metadata: Default::default(),
        name: Cow::Borrowed("ROOT"),
        class_name: ustr("ROOT"),
        properties: Default::default(),
        children: vec![InstanceSnapshot {
            snapshot_id: Ref::none(),
            metadata: Default::default(),
            name: Cow::Borrowed("New"),
            class_name: ustr("Folder"),
            properties: Default::default(),
            children: Vec::new(),
        }],
    };

    let patch_set = compute_patch_set(Some(snapshot), &tree, tree.get_root_id());
    let patch_value = redactions.redacted_yaml(patch_set);

    assert_yaml_snapshot!(patch_value);
}

#[test]
fn remove_child() {
    let mut redactions = RedactionMap::default();

    let mut tree = empty_tree();
    redactions.intern(tree.get_root_id());

    {
        let root_id = tree.get_root_id();
        let new_id = tree.insert_instance(
            root_id,
            InstanceSnapshot::new().name("Should not appear in snapshot"),
        );

        redactions.intern(new_id);
    }

    let snapshot = InstanceSnapshot {
        snapshot_id: Ref::none(),
        metadata: Default::default(),
        name: Cow::Borrowed("ROOT"),
        class_name: ustr("ROOT"),
        properties: Default::default(),
        children: Vec::new(),
    };

    let patch_set = compute_patch_set(Some(snapshot), &tree, tree.get_root_id());
    let patch_value = redactions.redacted_yaml(patch_set);

    assert_yaml_snapshot!(patch_value);
}

fn empty_tree() -> RojoTree {
    RojoTree::new(InstanceSnapshot::new().name("ROOT").class_name("ROOT"))
}
