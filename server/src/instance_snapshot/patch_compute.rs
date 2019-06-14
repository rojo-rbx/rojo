use std::collections::{HashMap, HashSet};

use rbx_dom_weak::{RbxTree, RbxId, RbxInstance};

use super::{
    snapshot::InstanceSnapshot,
    patch::{PatchSet, PatchChildren, PatchChildrenEntry, PatchProperties},
};

pub fn compute_patch<'a>(
    snapshot: &'a InstanceSnapshot,
    tree: &RbxTree,
    id: RbxId,
    patch_set: &mut PatchSet<'a>,
) {
    let instance = tree.get_instance(id)
        .expect("Instance did not exist in tree");

    if instance.class_name != snapshot.class_name {
        panic!("NYI: changing class name of an instance?");
    }

    compute_property_patch(snapshot, instance, patch_set);
    compute_children_patch(snapshot, tree, id, patch_set);
}

fn compute_children_patch<'a>(
    snapshot: &'a InstanceSnapshot,
    tree: &RbxTree,
    id: RbxId,
    patch_set: &mut PatchSet<'a>,
) {
    let instance = tree.get_instance(id)
        .expect("Instance did not exist in tree");

    let instance_children = instance.get_children_ids();

    let mut has_changes = false;
    let mut paired_instances = vec![false; instance_children.len()];

    let children: Vec<PatchChildrenEntry> = snapshot.children
        .iter()
        .enumerate()
        .map(|(snapshot_index, snapshot_child)| {
            let matching_instance = instance_children
                .iter()
                .enumerate()
                .find(|(instance_index, instance_child_id)| {
                    if paired_instances[*instance_index] {
                        return false;
                    }

                    let instance_child = tree.get_instance(**instance_child_id)
                        .expect("Instance did not exist in tree");

                    if snapshot_child.name == instance_child.name && instance_child.class_name == instance_child.class_name {
                        paired_instances[*instance_index] = true;

                        return true;
                    }

                    false
                });

            match matching_instance {
                Some((_, instance_child_id)) => {
                    compute_patch(snapshot_child, tree, *instance_child_id, patch_set);

                    PatchChildrenEntry::Existing(*instance_child_id)
                }
                None => {
                    has_changes = true;
                    PatchChildrenEntry::Added(snapshot_child.clone())
                }
            }
        })
        .collect();

    let mut removed_children = Vec::new();

    for (instance_index, instance_child_id) in instance_children.iter().enumerate() {
        if paired_instances[instance_index] {
            continue;
        }

        removed_children.push(*instance_child_id);
        has_changes = true;
    }

    if !has_changes {
        return;
    }

    patch_set.children.push(PatchChildren {
        id,
        children,
        removed_children,
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