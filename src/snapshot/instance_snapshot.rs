//! Defines the structure of an instance snapshot.

use std::{borrow::Cow, collections::HashMap};

use rbx_dom_weak::{RbxId, RbxTree, RbxValue};
use serde::{Deserialize, Serialize};

use super::InstanceMetadata;

/// A lightweight description of what an instance should look like.
///
// Possible future improvements:
// - Use refcounted/interned strings
// - Replace use of RbxValue with a sum of RbxValue + borrowed value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstanceSnapshot {
    /// A temporary ID applied to the snapshot that's used for Ref properties.
    pub snapshot_id: Option<RbxId>,

    /// Rojo-specific metadata associated with the instance.
    pub metadata: InstanceMetadata,

    /// Correpsonds to the Name property of the instance.
    pub name: Cow<'static, str>,

    /// Corresponds to the ClassName property of the instance.
    pub class_name: Cow<'static, str>,

    /// All other properties of the instance, weakly-typed.
    pub properties: HashMap<String, RbxValue>,

    /// The children of the instance represented as more snapshots.
    ///
    /// Order is relevant for Roblox instances!
    pub children: Vec<InstanceSnapshot>,
}

impl InstanceSnapshot {
    pub fn from_tree(tree: &RbxTree, id: RbxId) -> InstanceSnapshot {
        let instance = tree
            .get_instance(id)
            .expect("instance did not exist in tree");

        let children = instance
            .get_children_ids()
            .iter()
            .cloned()
            .map(|id| InstanceSnapshot::from_tree(tree, id))
            .collect();

        InstanceSnapshot {
            snapshot_id: Some(id),
            metadata: InstanceMetadata::default(),
            name: Cow::Owned(instance.name.clone()),
            class_name: Cow::Owned(instance.class_name.clone()),
            properties: instance.properties.clone(),
            children,
        }
    }
}

impl Default for InstanceSnapshot {
    fn default() -> InstanceSnapshot {
        InstanceSnapshot {
            snapshot_id: None,
            metadata: InstanceMetadata::default(),
            name: Cow::Borrowed("DEFAULT"),
            class_name: Cow::Borrowed("DEFAULT"),
            properties: HashMap::new(),
            children: Vec::new(),
        }
    }
}
