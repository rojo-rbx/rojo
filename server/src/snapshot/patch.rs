//! Defines the data structures used for describing instance patches.

use std::collections::HashMap;

use rbx_dom_weak::{RbxId, RbxValue};

use super::InstanceSnapshot;

/// A set of different kinds of patches that can be applied to an RbxTree.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct PatchSet<'a> {
    pub removed_instances: Vec<RbxId>,
    pub added_instances: Vec<PatchAddInstance<'a>>,
    pub updated_instances: Vec<PatchUpdateInstance>,
}

impl<'a> PatchSet<'a> {
    pub fn new() -> PatchSet<'a> {
        PatchSet {
            removed_instances: Vec::new(),
            added_instances: Vec::new(),
            updated_instances: Vec::new(),
        }
    }
}

/// A patch containing an instance that was added to the tree.
#[derive(Debug, Clone, PartialEq)]
pub struct PatchAddInstance<'a> {
    pub parent_id: RbxId,
    pub instance: InstanceSnapshot<'a>,
}

/// A patch indicating that properties (or the name) of an instance changed.
#[derive(Debug, Clone, PartialEq)]
pub struct PatchUpdateInstance {
    pub id: RbxId,
    pub changed_name: Option<String>,
    pub changed_class_name: Option<String>,

    /// Contains all changed properties. If a property is assigned to `None`,
    /// then that property has been removed.
    pub changed_properties: HashMap<String, Option<RbxValue>>,
}
