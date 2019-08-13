//! Defines the algorithm for applying generated patches.

use std::collections::HashMap;

use rbx_dom_weak::{RbxTree, RbxValue, RbxId, RbxInstanceProperties};

use super::{
    patch::{PatchSet, PatchUpdateInstance},
    InstanceSnapshot,
};

pub fn apply_patch_set(
    tree: &mut RbxTree,
    patch_set: &PatchSet,
) {
    let mut context = PatchApplyContext::default();

    for removed_id in &patch_set.removed_instances {
        tree.remove_instance(*removed_id);
    }

    for add_patch in &patch_set.added_instances {
        apply_add_child(&mut context, tree, add_patch.parent_id, &add_patch.instance);
    }

    for update_patch in &patch_set.updated_instances {
        apply_update_child(&context, tree, update_patch);
    }
}

#[derive(Default)]
struct PatchApplyContext {
    snapshot_id_to_instance_id: HashMap<RbxId, RbxId>,
}

fn apply_add_child(
    context: &mut PatchApplyContext,
    tree: &mut RbxTree,
    parent_id: RbxId,
    snapshot: &InstanceSnapshot,
) {
    let properties = RbxInstanceProperties {
        name: snapshot.name.clone().into_owned(),
        class_name: snapshot.class_name.clone().into_owned(),

        // TODO: Apply no properties here and defer their application to later.
        properties: snapshot.properties.clone(),
    };

    let id = tree.insert_instance(properties, parent_id);

    if let Some(snapshot_id) = snapshot.snapshot_id {
        context.snapshot_id_to_instance_id.insert(snapshot_id, id);
    }

    for child_snapshot in &snapshot.children {
        apply_add_child(context, tree, id, child_snapshot);
    }
}

fn apply_update_child(
    context: &PatchApplyContext,
    tree: &mut RbxTree,
    patch: &PatchUpdateInstance,
) {
    let instance = tree.get_instance_mut(patch.id)
        .expect("Instance referred to by patch does not exist");

    if let Some(name) = &patch.changed_name {
        instance.name = name.clone();
    }

    if let Some(class_name) = &patch.changed_class_name {
        instance.class_name = class_name.clone();
    }

    for (key, property_entry) in &patch.changed_properties {
        match property_entry {
            // Ref values need to be potentially rewritten from snapshot IDs to
            // instance IDs if they referred to an instance that was created as
            // part of this patch.
            Some(RbxValue::Ref { value: Some(id) }) => {
                let new_id = context.snapshot_id_to_instance_id.get(id).unwrap_or(id);

                instance.properties.insert(key.clone(), RbxValue::Ref {
                    value: Some(*new_id),
                });
            }
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

    use maplit::hashmap;
    use rbx_dom_weak::RbxValue;

    #[test]
    fn add_from_empty() {
        let _ = env_logger::try_init();

        let mut tree = RbxTree::new(RbxInstanceProperties {
            name: "Folder".to_owned(),
            class_name: "Folder".to_owned(),
            properties: HashMap::new(),
        });

        let root_id = tree.get_root_id();

        let snapshot = InstanceSnapshot {
            snapshot_id: None,
            name: Cow::Borrowed("Foo"),
            class_name: Cow::Borrowed("Bar"),
            properties: hashmap! {
                "Baz".to_owned() => RbxValue::Int32 { value: 5 },
            },
            children: Vec::new(),
        };

        apply_add_child(&mut PatchApplyContext::default(), &mut tree, root_id, &snapshot);

        let root_instance = tree.get_instance(root_id).unwrap();
        let child_id = root_instance.get_children_ids()[0];
        let child_instance = tree.get_instance(child_id).unwrap();

        assert_eq!(child_instance.name.as_str(), &snapshot.name);
        assert_eq!(child_instance.class_name.as_str(), &snapshot.class_name);
        assert_eq!(&child_instance.properties, &snapshot.properties);
        assert!(child_instance.get_children_ids().is_empty());
    }

    #[test]
    fn update_existing() {
        let _ = env_logger::try_init();

        let mut tree = RbxTree::new(RbxInstanceProperties {
            name: "OldName".to_owned(),
            class_name: "OldClassName".to_owned(),
            properties: hashmap! {
                "Foo".to_owned() => RbxValue::Int32 { value: 7 },
                "Bar".to_owned() => RbxValue::Int32 { value: 3 },
                "Unchanged".to_owned() => RbxValue::Int32 { value: -5 },
            },
        });

        let root_id = tree.get_root_id();

        let patch = PatchUpdateInstance {
            id: root_id,
            changed_name: Some("Foo".to_owned()),
            changed_class_name: Some("NewClassName".to_owned()),
            changed_properties: hashmap! {
                // The value of Foo has changed
                "Foo".to_owned() => Some(RbxValue::Int32 { value: 8 }),

                // Bar has been deleted
                "Bar".to_owned() => None,

                // Baz has been added
                "Baz".to_owned() => Some(RbxValue::Int32 { value: 10 }),
            },
        };

        apply_update_child(&PatchApplyContext::default(), &mut tree, &patch);

        let expected_properties = hashmap! {
            "Foo".to_owned() => RbxValue::Int32 { value: 8 },
            "Baz".to_owned() => RbxValue::Int32 { value: 10 },
            "Unchanged".to_owned() => RbxValue::Int32 { value: -5 },
        };

        let root_instance = tree.get_instance(root_id).unwrap();
        assert_eq!(root_instance.name, "Foo");
        assert_eq!(root_instance.class_name, "NewClassName");
        assert_eq!(root_instance.properties, expected_properties);
    }
}