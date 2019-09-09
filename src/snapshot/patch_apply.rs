//! Defines the algorithm for applying generated patches.

use std::collections::HashMap;

use rbx_dom_weak::{RbxId, RbxInstanceProperties, RbxValue};

use super::{
    patch::{PatchSet, PatchUpdateInstance},
    InstancePropertiesWithMeta, InstanceSnapshot, RojoTree,
};

pub fn apply_patch_set(tree: &mut RojoTree, patch_set: &PatchSet) {
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

    apply_deferred_properties(context, tree);
}

#[derive(Default)]
struct PatchApplyContext {
    snapshot_id_to_instance_id: HashMap<RbxId, RbxId>,
    properties_to_apply: HashMap<RbxId, HashMap<String, RbxValue>>,
}

/// Apply properties that were deferred in order to get more information.
///
/// Ref properties from snapshots refer to eachother via snapshot ID. Some of
/// these properties are transformed when the patch is computed, notably the
/// instances that the patch computing method is able to pair up.
///
/// The remaining Ref properties need to be handled during patch application,
/// where we build up a map of snapshot IDs to instance IDs as they're created,
/// then apply properties all at once at the end.
fn apply_deferred_properties(context: PatchApplyContext, tree: &mut RojoTree) {
    for (id, mut properties) in context.properties_to_apply {
        let instance = tree
            .get_instance_mut(id)
            .expect("Invalid instance ID in deferred property map");

        for property_value in properties.values_mut() {
            if let RbxValue::Ref { value: Some(id) } = property_value {
                if let Some(&instance_id) = context.snapshot_id_to_instance_id.get(id) {
                    *property_value = RbxValue::Ref {
                        value: Some(instance_id),
                    };
                }
            }
        }

        instance.instance.properties = properties;
    }
}

fn apply_add_child(
    context: &mut PatchApplyContext,
    tree: &mut RojoTree,
    parent_id: RbxId,
    snapshot: &InstanceSnapshot,
) {
    let properties = InstancePropertiesWithMeta {
        properties: RbxInstanceProperties {
            name: snapshot.name.clone().into_owned(),
            class_name: snapshot.class_name.clone().into_owned(),

            // Property assignment is deferred until after we know about all
            // instances in this patch.
            properties: HashMap::new(),
        },
        metadata: Default::default(), // TODO
    };

    let id = tree.insert_instance(properties, parent_id);

    context
        .properties_to_apply
        .insert(id, snapshot.properties.clone());

    if let Some(snapshot_id) = snapshot.snapshot_id {
        context.snapshot_id_to_instance_id.insert(snapshot_id, id);
    }

    for child_snapshot in &snapshot.children {
        apply_add_child(context, tree, id, child_snapshot);
    }
}

fn apply_update_child(
    context: &PatchApplyContext,
    tree: &mut RojoTree,
    patch: &PatchUpdateInstance,
) {
    let instance = tree
        .get_instance_mut(patch.id)
        .expect("Instance referred to by patch does not exist");

    if let Some(name) = &patch.changed_name {
        instance.instance.name = name.clone();
    }

    if let Some(class_name) = &patch.changed_class_name {
        instance.instance.class_name = class_name.clone();
    }

    for (key, property_entry) in &patch.changed_properties {
        match property_entry {
            // Ref values need to be potentially rewritten from snapshot IDs to
            // instance IDs if they referred to an instance that was created as
            // part of this patch.
            Some(RbxValue::Ref { value: Some(id) }) => {
                let new_id = context.snapshot_id_to_instance_id.get(id).unwrap_or(id);

                instance.instance.properties.insert(
                    key.clone(),
                    RbxValue::Ref {
                        value: Some(*new_id),
                    },
                );
            }
            Some(value) => {
                instance
                    .instance
                    .properties
                    .insert(key.clone(), value.clone());
            }
            None => {
                instance.instance.properties.remove(key);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::{borrow::Cow, collections::HashMap};

    use maplit::hashmap;
    use rbx_dom_weak::RbxValue;

    use super::super::PatchAddInstance;

    #[test]
    fn add_from_empty() {
        let _ = env_logger::try_init();

        let mut tree = RojoTree::new(InstancePropertiesWithMeta {
            properties: RbxInstanceProperties {
                name: "Folder".to_owned(),
                class_name: "Folder".to_owned(),
                properties: HashMap::new(),
            },
            metadata: Default::default(),
        });

        let root_id = tree.get_root_id();

        let snapshot = InstanceSnapshot {
            snapshot_id: None,
            metadata: Default::default(),
            name: Cow::Borrowed("Foo"),
            class_name: Cow::Borrowed("Bar"),
            properties: hashmap! {
                "Baz".to_owned() => RbxValue::Int32 { value: 5 },
            },
            children: Vec::new(),
        };

        let patch_set = PatchSet {
            added_instances: vec![PatchAddInstance {
                parent_id: root_id,
                instance: snapshot.clone(),
            }],
            ..Default::default()
        };

        apply_patch_set(&mut tree, &patch_set);

        let root_instance = tree.get_instance(root_id).unwrap();
        let child_id = root_instance.instance.get_children_ids()[0];
        let child_instance = tree.get_instance(child_id).unwrap();

        assert_eq!(child_instance.instance.name.as_str(), &snapshot.name);
        assert_eq!(
            child_instance.instance.class_name.as_str(),
            &snapshot.class_name
        );
        assert_eq!(&child_instance.instance.properties, &snapshot.properties);
        assert!(child_instance.instance.get_children_ids().is_empty());
    }

    #[test]
    fn update_existing() {
        let _ = env_logger::try_init();

        let mut tree = RojoTree::new(InstancePropertiesWithMeta {
            properties: RbxInstanceProperties {
                name: "OldName".to_owned(),
                class_name: "OldClassName".to_owned(),
                properties: hashmap! {
                    "Foo".to_owned() => RbxValue::Int32 { value: 7 },
                    "Bar".to_owned() => RbxValue::Int32 { value: 3 },
                    "Unchanged".to_owned() => RbxValue::Int32 { value: -5 },
                },
            },
            metadata: Default::default(),
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
            changed_metadata: None,
        };

        let patch_set = PatchSet {
            updated_instances: vec![patch],
            ..Default::default()
        };

        apply_patch_set(&mut tree, &patch_set);

        let expected_properties = hashmap! {
            "Foo".to_owned() => RbxValue::Int32 { value: 8 },
            "Baz".to_owned() => RbxValue::Int32 { value: 10 },
            "Unchanged".to_owned() => RbxValue::Int32 { value: -5 },
        };

        let root_instance = tree.get_instance(root_id).unwrap();
        assert_eq!(root_instance.instance.name, "Foo");
        assert_eq!(root_instance.instance.class_name, "NewClassName");
        assert_eq!(root_instance.instance.properties, expected_properties);
    }
}
