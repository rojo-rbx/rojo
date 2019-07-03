use std::collections::HashMap;

use rbx_dom_weak::{RbxValue, RbxId};

use super::snapshot::InstanceSnapshot;

/// A set of different kinds of patches that can be applied to an RbxTree.
#[derive(Debug, Clone)]
pub struct PatchSet<'a> {
    pub removed_instances: Vec<RbxId>,
    pub added_instances: Vec<PatchAddInstance<'a>>,
    pub updated_instances: Vec<PatchUpdateInstance>,
}

#[derive(Debug, Clone)]
pub struct PatchAddInstance<'a> {
    pub parent_id: RbxId,
    pub instance: InstanceSnapshot<'a>,
}

/// A patch indicating that properties (or the name) of an instance changed.
#[derive(Debug, Clone)]
pub struct PatchUpdateInstance {
    pub id: RbxId,
    pub changed_name: Option<String>,
    pub changed_properties: HashMap<String, Option<RbxValue>>,
}