use std::collections::HashMap;

use rbx_dom_weak::{RbxValue, RbxId};

use super::snapshot::InstanceSnapshot;

pub struct PatchSet<'a> {
    pub children: Vec<PatchChildren<'a>>,
    pub properties: Vec<PatchProperties>,
}

pub struct PatchChildren<'a> {
    pub id: RbxId,
    pub children: Vec<RbxId>,
    pub added_children: Vec<InstanceSnapshot<'a>>,
    pub removed_children: Vec<RbxId>,
}

pub struct PatchProperties {
    pub id: RbxId,
    pub changed_name: Option<String>,
    pub changed_properties: HashMap<String, Option<RbxValue>>,
}