use std::collections::HashMap;

use rbx_dom_weak::{RbxValue, RbxId};

use super::snapshot::InstanceSnapshot;

#[derive(Debug, Clone)]
pub struct PatchSet<'a> {
    pub children: Vec<PatchChildren<'a>>,
    pub properties: Vec<PatchProperties>,
    pub id_map: HashMap<RbxId, RbxId>,
}

#[derive(Debug, Clone)]
pub struct PatchChildren<'a> {
    pub id: RbxId,
    pub children: Vec<PatchChildrenEntry<'a>>,
}

#[derive(Debug, Clone)]
pub enum PatchChildrenEntry<'a> {
    Existing(RbxId),
    Added(InstanceSnapshot<'a>),
}

#[derive(Debug, Clone)]
pub struct PatchProperties {
    pub id: RbxId,
    pub changed_name: Option<String>,
    pub changed_properties: HashMap<String, Option<RbxValue>>,
}