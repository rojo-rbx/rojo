use std::collections::HashMap;

use rbx_dom_weak::{RbxValue, RbxId};

use super::snapshot::InstanceSnapshot;

/// A set of different kinds of patches that can be applied to an RbxTree.
#[derive(Debug, Clone)]
pub struct PatchSet<'a> {
    pub children: Vec<PatchChildren<'a>>,
    pub properties: Vec<PatchProperties>,

    /// I don't remember what this property was intended for, maybe for dealing
    /// with Ref properties?
    pub id_map: HashMap<RbxId, RbxId>,
}

/// A patch indicating that the children of an instance changed.
///
/// The given list of children should be the new list of children. Each entry in
/// the list is either an existing child (whose order in the child list may have
/// changed) or a new child that should be added.
///
/// Removed children can be inferred by comparing the children list against the
/// list of children that the instance currently has, but the struct also
/// contains a list of removed IDs for convenience.
///
/// Children are given this way because the order of children is relevant to the
/// functionality of an instance in Roblox. This constraint makes reconciliation
/// much more complicated in general.
#[derive(Debug, Clone)]
pub struct PatchChildren<'a> {
    pub id: RbxId,
    pub children: Vec<PatchChildrenEntry<'a>>,
    pub removed_children: Vec<RbxId>,
}

#[derive(Debug, Clone)]
pub enum PatchChildrenEntry<'a> {
    Existing(RbxId),
    Added(InstanceSnapshot<'a>),
}

/// A patch indicating that properties (or the name) of an instance changed.
#[derive(Debug, Clone)]
pub struct PatchProperties {
    pub id: RbxId,
    pub changed_name: Option<String>,
    pub changed_properties: HashMap<String, Option<RbxValue>>,
}