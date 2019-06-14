use std::collections::{HashMap, HashSet};

use rbx_dom_weak::{RbxTree, RbxId, RbxInstance};

use super::{
    snapshot::InstanceSnapshot,
    patch::{PatchSet, PatchChildren, PatchProperties},
};

pub fn compute_patch(
    snapshot: &InstanceSnapshot,
    tree: &RbxTree,
    id: RbxId,
    patch_set: &mut PatchSet,
) {
    let instance = tree.get_instance(id)
        .expect("Instance did not exist in tree");

    if instance.class_name != snapshot.class_name {
        panic!("NYI: changing class name of an instance?");
    }

    compute_property_patch(snapshot, instance, patch_set);
    compute_children_patch(snapshot, tree, id, patch_set);
}

fn compute_children_patch(
    snapshot: &InstanceSnapshot,
    tree: &RbxTree,
    id: RbxId,
    patch_set: &mut PatchSet,
) {
    let instance = tree.get_instance(id)
        .expect("Instance did not exist in tree");

    let instance_children = instance.get_children_ids();

    let mut added_children = Vec::new();
    let mut removed_children = Vec::new();

    let mut paired_instances = vec![false; instance_children.len()];
    let mut paired_snapshots = vec![false; snapshot.children.len()];

    for (snapshot_index, snapshot_child) in snapshot.children.iter().enumerate() {
        let mut matching_instance = None;
        for (instance_index, instance_child_id) in instance_children.iter().enumerate() {
            if paired_instances[instance_index] {
                continue;
            }

            let instance_child = tree.get_instance(instance_child_id)
                .expect("Instance did not exist in tree");

            if snapshot_child.name == instance_child.name {
                matching_instance = Some(instance_child);
                paired_instances[instance_index] = true;
                paired_snapshots[snapshot_index] = true;

                break;
            }
        }

        match matching_instance {
            Some(instance_child) => {
                compute_patch(snapshot_child, tree, instance_child.get_id(), patch_set);
            }
            None => {
                added_children.push(snapshot_child.clone());
            }
        }
    }

    for (instance_index, instance_child_id) in instance_children.iter().enumerate() {
        if paired_instances[instance_index] {
            continue;
        }

        removed_children.push(instance_child_id);
    }

    if added_children.is_empty() && removed_children.is_empty() {
        return;
    }

    patch_set.children.push(PatchChildren {
        id,
        children: snapshot.children.clone(),
        // ???
    });
}

fn compute_property_patch(
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

    for (name, instance_value) in &instance.properties {
        if visited_properties.contains(name.as_str()) {
            continue;
        }

        changed_properties.insert(name.clone(), None);
    }

    if changed_properties.is_empty() && changed_name.is_none() {
        return;
    }

    patch_set.properties.push(PatchProperties {
        id: instance.get_id(),
        changed_name,
        changed_properties,
    });
}