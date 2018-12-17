use std::{
    str,
    borrow::Cow,
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use rbx_tree::{RbxTree, RbxId, RbxInstance, RbxValue};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstanceChanges {
    pub added: HashSet<RbxId>,
    pub removed: HashSet<RbxId>,
    pub updated: HashSet<RbxId>,
}

impl InstanceChanges {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.updated.is_empty()
    }
}

pub struct RbxSnapshotInstance<'a> {
    pub name: Cow<'a, str>,
    pub class_name: Cow<'a, str>,
    pub properties: HashMap<String, RbxValue>,
    pub children: Vec<RbxSnapshotInstance<'a>>,
    pub source_path: Option<PathBuf>,
}

pub fn reify_root(snapshot: &RbxSnapshotInstance, changes: &mut InstanceChanges) -> RbxTree {
    let instance = reify_core(snapshot);
    let mut tree = RbxTree::new(instance);
    let root_id = tree.get_root_id();

    changes.added.insert(root_id);

    for child in &snapshot.children {
        reify_subtree(child, &mut tree, root_id, changes);
    }

    tree
}

pub fn reify_subtree(snapshot: &RbxSnapshotInstance, tree: &mut RbxTree, parent_id: RbxId, changes: &mut InstanceChanges) {
    let instance = reify_core(snapshot);
    let id = tree.insert_instance(instance, parent_id);

    changes.added.insert(id);

    for child in &snapshot.children {
        reify_subtree(child, tree, id, changes);
    }
}

pub fn reconcile_subtree(tree: &mut RbxTree, id: RbxId, snapshot: &RbxSnapshotInstance, changes: &mut InstanceChanges) {
    if reconcile_instance_properties(tree.get_instance_mut(id).unwrap(), snapshot) {
        changes.updated.insert(id);
    }

    reconcile_instance_children(tree, id, snapshot, changes);
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

fn reconcile_instance_children(
    tree: &mut RbxTree,
    id: RbxId,
    snapshot: &RbxSnapshotInstance,
    changes: &mut InstanceChanges
) {
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
        reify_subtree(child_snapshot, tree, id, changes);
    }

    for child_id in &children_to_remove {
        if let Some(subtree) = tree.remove_instance(*child_id) {
            for id in subtree.iter_all_ids() {
                changes.removed.insert(id);
            }
        }
    }

    for (child_id, child_snapshot) in &children_to_update {
        reconcile_subtree(tree, *child_id, child_snapshot, changes);
    }
}