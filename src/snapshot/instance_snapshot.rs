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
    pub fn new() -> Self {
        Self {
            snapshot_id: None,
            metadata: InstanceMetadata::default(),
            name: Cow::Borrowed("DEFAULT"),
            class_name: Cow::Borrowed("DEFAULT"),
            properties: HashMap::new(),
            children: Vec::new(),
        }
    }

    pub fn name(self, name: impl Into<String>) -> Self {
        Self {
            name: Cow::Owned(name.into()),
            ..self
        }
    }

    pub fn class_name(self, class_name: impl Into<String>) -> Self {
        Self {
            class_name: Cow::Owned(class_name.into()),
            ..self
        }
    }

    pub fn properties(self, properties: impl Into<HashMap<String, RbxValue>>) -> Self {
        Self {
            properties: properties.into(),
            ..self
        }
    }

    pub fn children(self, children: impl Into<Vec<Self>>) -> Self {
        Self {
            children: children.into(),
            ..self
        }
    }

    pub fn metadata(self, metadata: impl Into<InstanceMetadata>) -> Self {
        Self {
            metadata: metadata.into(),
            ..self
        }
    }

    pub fn from_tree(tree: &RbxTree, id: RbxId) -> Self {
        let instance = tree
            .get_instance(id)
            .expect("instance did not exist in tree");

        let children = instance
            .get_children_ids()
            .iter()
            .cloned()
            .map(|id| Self::from_tree(tree, id))
            .collect();

        Self {
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
    fn default() -> Self {
        Self::new()
    }
}
