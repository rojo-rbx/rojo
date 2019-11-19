//! Defines the algorithm for computing a roughly-minimal patch set given an
//! existing instance tree and an instance snapshot.

use std::collections::{HashMap, HashSet};

use rbx_dom_weak::{RbxId, RbxValue};

use super::{
    patch::{PatchAdd, PatchSet, PatchUpdate},
    InstanceSnapshot, InstanceWithMeta, RojoTree,
};

pub fn compute_patch_set(snapshot: &InstanceSnapshot, tree: &RojoTree, id: RbxId) -> PatchSet {
    let mut patch_set = PatchSet::new();
    let mut context = ComputePatchContext::default();

    compute_patch_set_internal(&mut context, snapshot, tree, id, &mut patch_set);

    // Rewrite Ref properties to refer to instance IDs instead of snapshot IDs
    // for all of the IDs that we know about so far.
    rewrite_refs_in_updates(&context, &mut patch_set.updated_instances);
    rewrite_refs_in_additions(&context, &mut patch_set.added_instances);

    patch_set
}

#[derive(Default)]
struct ComputePatchContext {
    snapshot_id_to_instance_id: HashMap<RbxId, RbxId>,
}

fn rewrite_refs_in_updates(context: &ComputePatchContext, updates: &mut [PatchUpdate]) {
    for update in updates {
        for property_value in update.changed_properties.values_mut() {
            if let Some(RbxValue::Ref { value: Some(id) }) = property_value {
                if let Some(&instance_id) = context.snapshot_id_to_instance_id.get(id) {
                    *property_value = Some(RbxValue::Ref {
                        value: Some(instance_id),
                    });
                }
            }
        }
    }
}

fn rewrite_refs_in_additions(context: &ComputePatchContext, additions: &mut [PatchAdd]) {
    for addition in additions {
        rewrite_refs_in_snapshot(context, &mut addition.instance);
    }
}

fn rewrite_refs_in_snapshot(context: &ComputePatchContext, snapshot: &mut InstanceSnapshot) {
    for property_value in snapshot.properties.values_mut() {
        if let RbxValue::Ref { value: Some(id) } = property_value {
            if let Some(&instance_id) = context.snapshot_id_to_instance_id.get(id) {
                *property_value = RbxValue::Ref {
                    value: Some(instance_id),
                };
            }
        }
    }

    for child in &mut snapshot.children {
        rewrite_refs_in_snapshot(context, child);
    }
}

fn compute_patch_set_internal(
    context: &mut ComputePatchContext,
    snapshot: &InstanceSnapshot,
    tree: &RojoTree,
    id: RbxId,
    patch_set: &mut PatchSet,
) {
    if let Some(snapshot_id) = snapshot.snapshot_id {
        context.snapshot_id_to_instance_id.insert(snapshot_id, id);
    }

    let instance = tree
        .get_instance(id)
        .expect("Instance did not exist in tree");

    compute_property_patches(snapshot, &instance, patch_set);
    compute_children_patches(context, snapshot, tree, id, patch_set);
}

fn compute_property_patches(
    snapshot: &InstanceSnapshot,
    instance: &InstanceWithMeta,
    patch_set: &mut PatchSet,
) {
    let mut visited_properties = HashSet::new();
    let mut changed_properties = HashMap::new();

    let changed_name = if snapshot.name == instance.name() {
        None
    } else {
        Some(snapshot.name.clone().into_owned())
    };

    let changed_class_name = if snapshot.class_name == instance.class_name() {
        None
    } else {
        Some(snapshot.class_name.clone().into_owned())
    };

    let changed_metadata = if &snapshot.metadata == instance.metadata() {
        None
    } else {
        Some(snapshot.metadata.clone())
    };

    for (name, snapshot_value) in &snapshot.properties {
        visited_properties.insert(name.as_str());

        match instance.properties().get(name) {
            Some(instance_value) => {
                if snapshot_value != instance_value {
                    changed_properties.insert(name.clone(), Some(snapshot_value.clone()));
                }
            }
            None => {
                changed_properties.insert(name.clone(), Some(snapshot_value.clone()));
            }
        }
    }

    for name in instance.properties().keys() {
        if visited_properties.contains(name.as_str()) {
            continue;
        }

        changed_properties.insert(name.clone(), None);
    }

    if changed_properties.is_empty()
        && changed_name.is_none()
        && changed_class_name.is_none()
        && changed_metadata.is_none()
    {
        return;
    }

    patch_set.updated_instances.push(PatchUpdate {
        id: instance.id(),
        changed_name,
        changed_class_name,
        changed_properties,
        changed_metadata,
    });
}

fn compute_children_patches(
    context: &mut ComputePatchContext,
    snapshot: &InstanceSnapshot,
    tree: &RojoTree,
    id: RbxId,
    patch_set: &mut PatchSet,
) {
    let instance = tree
        .get_instance(id)
        .expect("Instance did not exist in tree");

    let instance_children = instance.children();

    let mut paired_instances = vec![false; instance_children.len()];

    for snapshot_child in snapshot.children.iter() {
        let matching_instance =
            instance_children
                .iter()
                .enumerate()
                .find(|(instance_index, instance_child_id)| {
                    if paired_instances[*instance_index] {
                        return false;
                    }

                    let instance_child = tree
                        .get_instance(**instance_child_id)
                        .expect("Instance did not exist in tree");

                    if snapshot_child.name == instance_child.name()
                        && snapshot_child.class_name == instance_child.class_name()
                    {
                        paired_instances[*instance_index] = true;
                        return true;
                    }

                    false
                });

        match matching_instance {
            Some((_, instance_child_id)) => {
                compute_patch_set_internal(
                    context,
                    snapshot_child,
                    tree,
                    *instance_child_id,
                    patch_set,
                );
            }
            None => {
                patch_set.added_instances.push(PatchAdd {
                    parent_id: id,
                    instance: snapshot_child.clone(),
                });
            }
        }
    }

    for (instance_index, instance_child_id) in instance_children.iter().enumerate() {
        if paired_instances[instance_index] {
            continue;
        }

        patch_set.removed_instances.push(*instance_child_id);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::borrow::Cow;

    use maplit::hashmap;
    use rbx_dom_weak::RbxInstanceProperties;

    use super::super::InstancePropertiesWithMeta;

    /// This test makes sure that rewriting refs in instance update patches to
    /// instances that already exists works. We should be able to correlate the
    /// snapshot ID and instance ID during patch computation and replace the
    /// value before returning from compute_patch_set.
    #[test]
    fn rewrite_ref_existing_instance_update() {
        let tree = RojoTree::new(InstancePropertiesWithMeta {
            properties: RbxInstanceProperties {
                name: "foo".to_owned(),
                class_name: "foo".to_owned(),
                properties: HashMap::new(),
            },
            metadata: Default::default(),
        });

        let root_id = tree.get_root_id();

        // This snapshot should be identical to the existing tree except for the
        // addition of a prop named Self, which is a self-referential Ref.
        let snapshot_id = RbxId::new();
        let snapshot = InstanceSnapshot {
            snapshot_id: Some(snapshot_id),
            properties: hashmap! {
                "Self".to_owned() => RbxValue::Ref {
                    value: Some(snapshot_id),
                }
            },

            metadata: Default::default(),
            name: Cow::Borrowed("foo"),
            class_name: Cow::Borrowed("foo"),
            children: Vec::new(),
        };

        let patch_set = compute_patch_set(&snapshot, &tree, root_id);

        let expected_patch_set = PatchSet {
            updated_instances: vec![PatchUpdate {
                id: root_id,
                changed_name: None,
                changed_class_name: None,
                changed_properties: hashmap! {
                    "Self".to_owned() => Some(RbxValue::Ref {
                        value: Some(root_id),
                    }),
                },
                changed_metadata: None,
            }],
            added_instances: Vec::new(),
            removed_instances: Vec::new(),
        };

        assert_eq!(patch_set, expected_patch_set);
    }

    /// The same as rewrite_ref_existing_instance_update, except that the
    /// property is added in a new instance instead of modifying an existing
    /// one.
    #[test]
    fn rewrite_ref_existing_instance_addition() {
        let tree = RojoTree::new(InstancePropertiesWithMeta {
            properties: RbxInstanceProperties {
                name: "foo".to_owned(),
                class_name: "foo".to_owned(),
                properties: HashMap::new(),
            },
            metadata: Default::default(),
        });

        let root_id = tree.get_root_id();

        // This patch describes the existing instance with a new child added.
        let snapshot_id = RbxId::new();
        let snapshot = InstanceSnapshot {
            snapshot_id: Some(snapshot_id),
            children: vec![InstanceSnapshot {
                properties: hashmap! {
                    "Self".to_owned() => RbxValue::Ref {
                        value: Some(snapshot_id),
                    },
                },

                snapshot_id: None,
                metadata: Default::default(),
                name: Cow::Borrowed("child"),
                class_name: Cow::Borrowed("child"),
                children: Vec::new(),
            }],

            metadata: Default::default(),
            properties: HashMap::new(),
            name: Cow::Borrowed("foo"),
            class_name: Cow::Borrowed("foo"),
        };

        let patch_set = compute_patch_set(&snapshot, &tree, root_id);

        let expected_patch_set = PatchSet {
            added_instances: vec![PatchAdd {
                parent_id: root_id,
                instance: InstanceSnapshot {
                    snapshot_id: None,
                    metadata: Default::default(),
                    properties: hashmap! {
                        "Self".to_owned() => RbxValue::Ref {
                            value: Some(root_id),
                        },
                    },
                    name: Cow::Borrowed("child"),
                    class_name: Cow::Borrowed("child"),
                    children: Vec::new(),
                },
            }],
            updated_instances: Vec::new(),
            removed_instances: Vec::new(),
        };

        assert_eq!(patch_set, expected_patch_set);
    }
}
