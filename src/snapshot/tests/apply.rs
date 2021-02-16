use insta::assert_yaml_snapshot;
use maplit::hashmap;

use rojo_insta_ext::RedactionMap;

use crate::{
    snapshot::{apply_patch_set, InstanceSnapshot, PatchSet, PatchUpdate, RojoTree},
    tree_view::{intern_tree, view_tree},
};

#[test]
fn set_name_and_class_name() {
    let mut redactions = RedactionMap::new();

    let mut tree = empty_tree();
    intern_tree(&tree, &mut redactions);

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

    let tree_view = view_tree(&tree, &mut redactions);
    assert_yaml_snapshot!(tree_view);

    let applied_patch_value = redactions.redacted_yaml(applied_patch_set);
    assert_yaml_snapshot!(applied_patch_value);
}

#[test]
fn add_property() {
    let mut redactions = RedactionMap::new();

    let mut tree = empty_tree();
    intern_tree(&tree, &mut redactions);

    let patch_set = PatchSet {
        updated_instances: vec![PatchUpdate {
            id: tree.get_root_id(),
            changed_name: None,
            changed_class_name: None,
            changed_properties: hashmap! {
                "Foo".to_owned() => Some("Value of Foo".into()),
            },
            changed_metadata: None,
        }],
        ..Default::default()
    };

    let applied_patch_set = apply_patch_set(&mut tree, patch_set);

    let tree_view = view_tree(&tree, &mut redactions);
    assert_yaml_snapshot!(tree_view);

    let applied_patch_value = redactions.redacted_yaml(applied_patch_set);
    assert_yaml_snapshot!(applied_patch_value);
}

#[test]
fn remove_property() {
    let mut redactions = RedactionMap::new();

    let mut tree = empty_tree();
    intern_tree(&tree, &mut redactions);

    {
        let root_id = tree.get_root_id();
        let mut root_instance = tree.get_instance_mut(root_id).unwrap();

        root_instance
            .properties_mut()
            .insert("Foo".to_owned(), "Should be removed".into());
    }

    let tree_view = view_tree(&tree, &mut redactions);
    assert_yaml_snapshot!("remove_property_initial", tree_view);

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

    let tree_view = view_tree(&tree, &mut redactions);
    assert_yaml_snapshot!("remove_property_after_patch", tree_view);

    let applied_patch_value = redactions.redacted_yaml(applied_patch_set);
    assert_yaml_snapshot!("remove_property_appied_patch", applied_patch_value);
}

fn empty_tree() -> RojoTree {
    RojoTree::new(InstanceSnapshot::new().name("ROOT").class_name("ROOT"))
}
