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
    pub properties: HashMap<String, RbxSnapshotValue<'a>>,
    pub children: Vec<RbxSnapshotInstance<'a>>,
    pub update_trigger_paths: Vec<PathBuf>,
}

pub enum RbxSnapshotValue<'a> {
    String(Cow<'a, str>),
}

impl<'a> RbxSnapshotValue<'a> {
    pub fn to_rbx_value(&self) -> RbxValue {
        match self {
            RbxSnapshotValue::String(value) => RbxValue::String {
                value: value.to_string(),
            },
        }
    }
}

fn reify_core(snapshot: &RbxSnapshotInstance) -> RbxInstance {
    let mut properties = HashMap::new();

    for (key, value) in &snapshot.properties {
        properties.insert(key.clone(), value.to_rbx_value());
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
                if &snapshot_value.to_rbx_value() != instance_value {
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
                if &snapshot_value.to_rbx_value() != instance_value {
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
            Some(value) => instance.properties.insert(key, value.to_rbx_value()),
            None => instance.properties.remove(&key),
        };
    }

    has_diffs
}

fn reconcile_instance_children(tree: &mut RbxTree, id: RbxId, snapshot: &RbxSnapshotInstance, changed_ids: &mut Vec<RbxId>) {
    // TODO: enumerate and match up children, update props, construct and delete IDs
}

pub fn reconcile_subtree(tree: &mut RbxTree, id: RbxId, snapshot: &RbxSnapshotInstance, changed_ids: &mut Vec<RbxId>) {
    if reconcile_instance_properties(tree.get_instance_mut().unwrap(), snapshot) {
        changed_ids.push(id);
    }

    reconcile_instance_children(tree, id, snapshot, changed_ids);
}