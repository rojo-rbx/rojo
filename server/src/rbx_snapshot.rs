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