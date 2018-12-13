use std::{
    str,
    borrow::Cow,
    collections::HashMap,
    path::PathBuf,
};

use rbx_tree::{RbxTree, RbxId, RbxInstance, RbxValue};

pub struct RbxSnapshotInstance<'a> {
    pub name: Cow<'a, str>,
    pub class_name: Cow<'a, str>,
    pub properties: HashMap<String, RbxValue>,
    pub children: Vec<RbxSnapshotInstance<'a>>,
    pub update_trigger_paths: Vec<PathBuf>,
}

fn reify_core(snapshot: &RbxSnapshotInstance) -> RbxInstance {
    let mut properties = HashMap::new();

    for (key, value) in &snapshot.properties {
        properties.insert(key.clone(), value.clone());
    }

    let instance = RbxInstance {
        name: snapshot.name.to_string(),
        class_name: snapshot.class_name.to_string(),
        properties,
    };

    instance
}

pub fn reify_root(snapshot: &RbxSnapshotInstance) -> RbxTree {
    let instance = reify_core(snapshot);
    let mut tree = RbxTree::new(instance);
    let root_id = tree.get_root_id();

    for child in &snapshot.children {
        reify_child(child, &mut tree, root_id);
    }

    tree
}

fn reify_child(snapshot: &RbxSnapshotInstance, tree: &mut RbxTree, parent_id: RbxId) {
    let instance = reify_core(snapshot);
    let id = tree.insert_instance(instance, parent_id);

    for child in &snapshot.children {
        reify_child(child, tree, id);
    }
}

fn reconcile_instance_properties(instance: &mut RbxInstance, snapshot: &RbxSnapshotInstance) -> bool {
    let mut has_diffs = false;

    if instance.name != snapshot.name {
        instance.name = snapshot.name.to_string();
        has_diffs = true;
    }

    if instance.class_name != snapshot.class_name {
        instance.class_name = snapshot.class_name.to_string();
        has_diffs = true;
    }

    let mut property_updates = HashMap::new();

    for (key, instance_value) in &instance.properties {
        match snapshot.properties.get(key) {
            Some(snapshot_value) => {
                if snapshot_value != instance_value {
                    property_updates.insert(key.clone(), Some(snapshot_value.clone()));
                }
            },
            None => {
                property_updates.insert(key.clone(), None);
            },
        }
    }

    for (key, snapshot_value) in &snapshot.properties {
        if property_updates.contains_key(key) {
            continue;
        }

        match instance.properties.get(key) {
            Some(instance_value) => {
                if snapshot_value != instance_value {
                    property_updates.insert(key.clone(), Some(snapshot_value.clone()));
                }
            },
            None => {
                property_updates.insert(key.clone(), Some(snapshot_value.clone()));
            },
        }
    }

    has_diffs = has_diffs || !property_updates.is_empty();

    for (key, change) in property_updates.drain() {
        match change {
            Some(value) => instance.properties.insert(key, value),
            None => instance.properties.remove(&key),
        };
    }

    has_diffs
}

fn reconcile_instance_children(tree: &mut RbxTree, id: RbxId, snapshot: &RbxSnapshotInstance) {
    // TODO: enumerate and match up children, update props, construct and delete IDs
    let children_ids = tree.get_instance(id).unwrap().get_children_ids().to_vec();
    let child_count = children_ids.len().max(snapshot.children.len());

    let mut children_to_add = Vec::new();
    let mut children_to_update = Vec::new();
    let mut children_to_remove = Vec::new();

    for i in 0..child_count {
        let instance_child = children_ids
            .get(i)
            .map(|&id| tree.get_instance_mut(id).unwrap());
        let snapshot_child = snapshot.children.get(i);

        match (instance_child, snapshot_child) {
            (Some(instance_child), Some(snapshot_child)) => {
                children_to_update.push((instance_child.get_id(), snapshot_child));
            },
            (Some(instance_child), None) => {
                children_to_remove.push(instance_child.get_id());
            },
            (None, Some(snapshot_child)) => {
                children_to_add.push(snapshot_child);
            },
            (None, None) => unreachable!(),
        }
    }

    for child_snapshot in &children_to_add {
        reify_child(child_snapshot, tree, id);
    }

    for child_id in &children_to_remove {
        tree.remove_instance(*child_id);
    }

    for (child_id, child_snapshot) in &children_to_update {
        reconcile_subtree(tree, *child_id, child_snapshot);
    }
}

pub fn reconcile_subtree(tree: &mut RbxTree, id: RbxId, snapshot: &RbxSnapshotInstance) {
    reconcile_instance_properties(tree.get_instance_mut(id).unwrap(), snapshot);
    reconcile_instance_children(tree, id, snapshot);
}