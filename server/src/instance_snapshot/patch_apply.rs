use rbx_dom_weak::{RbxTree, RbxId, RbxInstanceProperties};

use super::{
    patch::{PatchSet, PatchUpdateInstance},
    snapshot::InstanceSnapshot,
};

pub fn apply_patch(
    tree: &mut RbxTree,
    patch_set: &PatchSet,
) {
    for removed_id in &patch_set.removed_instances {
        tree.remove_instance(*removed_id);
    }

    for add_patch in &patch_set.added_instances {
        apply_add_child(tree, add_patch.parent_id, &add_patch.instance);
    }

    for update_patch in &patch_set.updated_instances {
        apply_update_child(tree, update_patch);
    }
}

fn apply_add_child(
    tree: &mut RbxTree,
    parent_id: RbxId,
    snapshot: &InstanceSnapshot,
) {
    let properties = RbxInstanceProperties {
        name: snapshot.name.clone().into_owned(),
        class_name: snapshot.class_name.clone().into_owned(),
        properties: snapshot.properties.clone(),
    };

    let id = tree.insert_instance(properties, parent_id);

    for child_snapshot in &snapshot.children {
        apply_add_child(tree, id, child_snapshot);
    }
}

fn apply_update_child(
    tree: &mut RbxTree,
    patch: &PatchUpdateInstance,
) {
    let instance = tree.get_instance_mut(patch.id)
        .expect("Instance referred to by patch does not exist");

    if let Some(name) = &patch.changed_name {
        instance.name = name.clone();
    }

    for (key, property_entry) in &patch.changed_properties {
        match property_entry {
            Some(value) => {
                instance.properties.insert(key.clone(), value.clone());
            }
            None => {
                instance.properties.remove(key);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::{
        borrow::Cow,
        collections::HashMap,
    };

    use rbx_dom_weak::RbxValue;

    fn new_tree() -> (RbxTree, RbxId) {
        let tree = RbxTree::new(RbxInstanceProperties {
            name: "Folder".to_owned(),
            class_name: "Folder".to_owned(),
            properties: HashMap::new(),
        });

        let root_id = tree.get_root_id();

        (tree, root_id)
    }

    #[test]
    fn add_from_empty() {
        let (mut tree, root_id) = new_tree();

        let snapshot = InstanceSnapshot {
            snapshot_id: None,
            name: Cow::Borrowed("Foo"),
            class_name: Cow::Borrowed("Bar"),
            properties: {
                let mut map = HashMap::new();
                map.insert("Baz".to_owned(), RbxValue::Int32 {
                    value: 5,
                });
                map
            },
            children: Vec::new(),
        };

        apply_add_child(&mut tree, root_id, &snapshot);

        let root_instance = tree.get_instance(root_id).unwrap();
        let child_id = root_instance.get_children_ids()[0];
        let child_instance = tree.get_instance(child_id).unwrap();

        assert_eq!(child_instance.name.as_str(), &snapshot.name);
        assert_eq!(child_instance.class_name.as_str(), &snapshot.class_name);
        assert_eq!(&child_instance.properties, &snapshot.properties);
        assert!(child_instance.get_children_ids().is_empty());
    }
}