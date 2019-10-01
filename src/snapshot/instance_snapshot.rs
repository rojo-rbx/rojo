//! Defines the structure of an instance snapshot.

use std::{borrow::Cow, collections::HashMap};

use rbx_dom_weak::{RbxId, RbxTree, RbxValue};
use serde::{Deserialize, Serialize};

use super::InstanceMetadata;

/// A lightweight description of what an instance should look like. Attempts to
/// be somewhat memory efficient by borrowing from its source data, indicated by
/// the lifetime parameter `'source`.
///
// Possible future improvements:
// - Use refcounted/interned strings
// - Replace use of RbxValue with a sum of RbxValue + borrowed value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstanceSnapshot<'source> {
    /// A temporary ID applied to the snapshot that's used for Ref properties.
    pub snapshot_id: Option<RbxId>,

    /// Rojo-specific metadata associated with the instance.
    pub metadata: InstanceMetadata,

    /// Correpsonds to the Name property of the instance.
    pub name: Cow<'source, str>,

    /// Corresponds to the ClassName property of the instance.
    pub class_name: Cow<'source, str>,

    /// All other properties of the instance, weakly-typed.
    pub properties: HashMap<String, RbxValue>,

    /// The children of the instance represented as more snapshots.
    ///
    /// Order is relevant for Roblox instances!
    pub children: Vec<InstanceSnapshot<'source>>,
}

impl<'source> InstanceSnapshot<'source> {
    pub fn get_owned(&'source self) -> InstanceSnapshot<'static> {
        let children: Vec<InstanceSnapshot<'static>> = self
            .children
            .iter()
            .map(InstanceSnapshot::get_owned)
            .collect();

        InstanceSnapshot {
            snapshot_id: None,
            metadata: self.metadata.clone(),
            name: Cow::Owned(self.name.clone().into_owned()),
            class_name: Cow::Owned(self.class_name.clone().into_owned()),
            properties: self.properties.clone(),
            children,
        }
    }

    pub fn from_tree(tree: &RbxTree, id: RbxId) -> InstanceSnapshot<'static> {
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

impl<'source> Default for InstanceSnapshot<'source> {
    fn default() -> InstanceSnapshot<'source> {
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
