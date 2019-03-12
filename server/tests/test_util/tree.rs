//! Defines a mechanism to compare two RbxTree objects and generate a useful
//! diff if they aren't the same. These methods ignore IDs, which are randomly
//! generated whenever a tree is constructed anyways. This makes matching up
//! pairs of instances that should be the same pretty difficult.
//!
//! It relies on a couple different ideas:
//! - Instances with the same name and class name are matched as the same
//!   instance. See basic_equal for this logic
//! - A path of period-delimited names (like Roblox's GetFullName) should be
//!   enough to debug most issues. If it isn't, we can do something fun like
//!   generate GraphViz graphs.

use std::{
    borrow::Cow,
    collections::HashSet,
    fmt,
};

use rbx_dom_weak::{RbxId, RbxTree};

#[derive(Debug)]
pub struct TreeMismatch {
    pub path: Cow<'static, str>,
    pub detail: Cow<'static, str>,
}

impl TreeMismatch {
    pub fn new<'a, A: Into<Cow<'a, str>>, B: Into<Cow<'a, str>>>(path: A, detail: B) -> TreeMismatch {
        TreeMismatch {
            path: Cow::Owned(path.into().into_owned()),
            detail: Cow::Owned(detail.into().into_owned()),
        }
    }

    pub fn add_parent(mut self, name: &str) -> TreeMismatch {
        self.path.to_mut().insert(0, '.');
        self.path.to_mut().insert_str(0, name);

        TreeMismatch {
            path: self.path,
            detail: self.detail,
        }
    }
}

impl fmt::Display for TreeMismatch {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        writeln!(formatter, "Tree mismatch at path {}", self.path)?;
        writeln!(formatter, "{}", self.detail)
    }
}

pub fn trees_equal(left_tree: &RbxTree, right_tree: &RbxTree) -> Result<(), TreeMismatch> {
    let left_id = left_tree.get_root_id();
    let right_id = right_tree.get_root_id();

    basic_equal(left_tree, left_id, right_tree, right_id)?;
    properties_equal(left_tree, left_id, right_tree, right_id)?;

    children_equal(left_tree, left_id, right_tree, right_id)
        .map_err(|e| {
            let left_instance = left_tree.get_instance(left_id)
                .expect("ID did not exist in left tree");

            e.add_parent(&left_instance.name)
        })
}

fn basic_equal(left_tree: &RbxTree, left_id: RbxId, right_tree: &RbxTree, right_id: RbxId) -> Result<(), TreeMismatch> {
    let left_instance = left_tree.get_instance(left_id)
        .expect("ID did not exist in left tree");

    let right_instance = right_tree.get_instance(right_id)
        .expect("ID did not exist in right tree");

    if left_instance.name != right_instance.name {
        let message = format!("Name did not match ('{}' vs '{}')", left_instance.name, right_instance.name);

        return Err(TreeMismatch::new(&left_instance.name, message));
    }

    if left_instance.class_name != right_instance.class_name {
        let message = format!("Class name did not match ('{}' vs '{}')", left_instance.class_name, right_instance.class_name);

        return Err(TreeMismatch::new(&left_instance.name, message));
    }

    Ok(())
}

fn properties_equal(left_tree: &RbxTree, left_id: RbxId, right_tree: &RbxTree, right_id: RbxId) -> Result<(), TreeMismatch> {
    let left_instance = left_tree.get_instance(left_id)
        .expect("ID did not exist in left tree");

    let right_instance = right_tree.get_instance(right_id)
        .expect("ID did not exist in right tree");

    let mut visited = HashSet::new();

    for (key, left_value) in &left_instance.properties {
        visited.insert(key);

        let right_value = right_instance.properties.get(key);

        if Some(left_value) != right_value {
            let message = format!(
                "Property {}:\n\tLeft: {:?}\n\tRight: {:?}",
                key,
                Some(left_value),
                right_value,
            );

            return Err(TreeMismatch::new(&left_instance.name, message));
        }
    }

    for (key, right_value) in &right_instance.properties {
        if visited.contains(key) {
            continue;
        }

        let left_value = left_instance.properties.get(key);

        if left_value != Some(right_value) {
            let message = format!(
                "Property {} was {:?} on left and {:?} on right",
                key,
                left_value,
                Some(right_value),
            );

            return Err(TreeMismatch::new(&left_instance.name, message));
        }
    }

    Ok(())
}

fn children_equal(left_tree: &RbxTree, left_id: RbxId, right_tree: &RbxTree, right_id: RbxId) -> Result<(), TreeMismatch> {
    let left_instance = left_tree.get_instance(left_id)
        .expect("ID did not exist in left tree");

    let right_instance = right_tree.get_instance(right_id)
        .expect("ID did not exist in right tree");

    let left_children = left_instance.get_children_ids();
    let right_children = right_instance.get_children_ids();

    if left_children.len() != right_children.len() {
        return Err(TreeMismatch::new(&left_instance.name, "Instances had different numbers of children"));
    }

    for (left_child_id, right_child_id) in left_children.iter().zip(right_children) {
        basic_equal(left_tree, *left_child_id, right_tree, *right_child_id)
            .map_err(|e| e.add_parent(&left_instance.name))?;

        properties_equal(left_tree, *left_child_id, right_tree, *right_child_id)
            .map_err(|e| e.add_parent(&left_instance.name))?;

        children_equal(left_tree, *left_child_id, right_tree, *right_child_id)
            .map_err(|e| e.add_parent(&left_instance.name))?;
    }

    Ok(())
}