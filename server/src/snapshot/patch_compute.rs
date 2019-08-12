//! Defines the algorithm for computing a roughly-minimal patch set given an
//! existing instance tree and an instance snapshot.

use std::collections::{HashMap, HashSet};

use rbx_dom_weak::{RbxTree, RbxId, RbxInstance};

use super::{
    InstanceSnapshot,
    patch::{PatchSet, PatchAddInstance, PatchUpdateInstance},
};

pub fn compute_patch_set<'a>(
    snapshot: &'a InstanceSnapshot,
    tree: &RbxTree,
    id: RbxId,
) -> PatchSet<'a> {
    let mut patch_set = PatchSet::new();

    compute_patch_set_internal(snapshot, tree, id, &mut patch_set);

    patch_set
}

fn compute_patch_set_internal<'a>(
    snapshot: &'a InstanceSnapshot,
    tree: &RbxTree,
    id: RbxId,
    patch_set: &mut PatchSet<'a>,
){
    let instance = tree.get_instance(id)
        .expect("Instance did not exist in tree");

    compute_property_patches(snapshot, instance, patch_set);
    compute_children_patches(snapshot, tree, id, patch_set);
}

fn compute_property_patches(
    snapshot: &InstanceSnapshot,
    instance: &RbxInstance,
    patch_set: &mut PatchSet,
) {
    let mut visited_properties = HashSet::new();
    let mut changed_properties = HashMap::new();

    let changed_name = if snapshot.name == instance.name {
        None
    } else {
        Some(snapshot.name.clone().into_owned())
    };

    let changed_class_name = if snapshot.class_name == instance.class_name {
        None
    } else {
        Some(snapshot.class_name.clone().into_owned())
    };

    for (name, snapshot_value) in &snapshot.properties {
        visited_properties.insert(name.as_str());

        match instance.properties.get(name) {
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

    for name in instance.properties.keys() {
        if visited_properties.contains(name.as_str()) {
            continue;
        }

        changed_properties.insert(name.clone(), None);
    }

    if changed_properties.is_empty() && changed_name.is_none() {
        return;
    }

    patch_set.updated_instances.push(PatchUpdateInstance {
        id: instance.get_id(),
        changed_name,
        changed_class_name,
        changed_properties,
    });
}

fn compute_children_patches<'a>(
    snapshot: &'a InstanceSnapshot,
    tree: &RbxTree,
    id: RbxId,
    patch_set: &mut PatchSet<'a>,
) {
    let instance = tree.get_instance(id)
        .expect("Instance did not exist in tree");

    let instance_children = instance.get_children_ids();

    let mut paired_instances = vec![false; instance_children.len()];

    for snapshot_child in snapshot.children.iter() {
        let matching_instance = instance_children
            .iter()
            .enumerate()
            .find(|(instance_index, instance_child_id)| {
                if paired_instances[*instance_index] {
                    return false;
                }

                let instance_child = tree.get_instance(**instance_child_id)
                    .expect("Instance did not exist in tree");

                if snapshot_child.name == instance_child.name &&
                    instance_child.class_name == instance_child.class_name
                {
                    paired_instances[*instance_index] = true;
                    return true;
                }

                false
            });

        match matching_instance {
            Some((_, instance_child_id)) => {
                compute_patch_set_internal(snapshot_child, tree, *instance_child_id, patch_set);
            }
            None => {
                patch_set.added_instances.push(PatchAddInstance {
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