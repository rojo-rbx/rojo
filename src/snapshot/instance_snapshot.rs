//! Defines the structure of an instance snapshot.

use std::collections::HashMap;

use rbx_dom_weak::{
    types::{Ref, Variant},
    WeakDom,
};
use serde::{Deserialize, Serialize};

use crate::small_string::SmallString;

use super::InstanceMetadata;

/// A lightweight description of what an instance should look like.
///
// Possible future improvements:
// - Use refcounted/interned strings
// - Replace use of Variant with a sum of Variant + borrowed value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstanceSnapshot {
    // FIXME: Don't use Option<Ref> anymore!
    /// A temporary ID applied to the snapshot that's used for Ref properties.
    pub snapshot_id: Option<Ref>,

    /// Rojo-specific metadata associated with the instance.
    pub metadata: InstanceMetadata,

    /// Correpsonds to the Name property of the instance.
    pub name: SmallString,

    /// Corresponds to the ClassName property of the instance.
    pub class_name: SmallString,

    /// All other properties of the instance, weakly-typed.
    pub properties: HashMap<SmallString, Variant>,

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
            name: "DEFAULT".into(),
            class_name: "DEFAULT".into(),
            properties: HashMap::new(),
            children: Vec::new(),
        }
    }

    pub fn name(self, name: impl Into<SmallString>) -> Self {
        Self {
            name: name.into(),
            ..self
        }
    }

    pub fn class_name(self, class_name: impl Into<SmallString>) -> Self {
        Self {
            class_name: class_name.into(),
            ..self
        }
    }

    pub fn property<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<SmallString>,
        V: Into<Variant>,
    {
        self.properties.insert(key.into(), value.into());
        self
    }

    pub fn properties(self, properties: impl Into<HashMap<SmallString, Variant>>) -> Self {
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

    pub fn snapshot_id(self, snapshot_id: Option<Ref>) -> Self {
        Self {
            snapshot_id,
            ..self
        }
    }

    pub fn metadata(self, metadata: impl Into<InstanceMetadata>) -> Self {
        Self {
            metadata: metadata.into(),
            ..self
        }
    }

    pub fn from_tree(tree: &WeakDom, id: Ref) -> Self {
        let instance = tree.get_by_ref(id).expect("instance did not exist in tree");

        let children = instance
            .children()
            .iter()
            .copied()
            .map(|id| Self::from_tree(tree, id))
            .collect();

        let properties = instance
            .properties
            .iter()
            .map(|(key, value)| (key.into(), value.clone()))
            .collect();

        Self {
            snapshot_id: Some(id),
            metadata: InstanceMetadata::default(),
            name: SmallString::from(&instance.name),
            class_name: SmallString::from(&instance.class),
            properties,
            children,
        }
    }
}

impl Default for InstanceSnapshot {
    fn default() -> Self {
        Self::new()
    }
}
