//! Defines the structure of an instance snapshot.

use std::{borrow::Cow, collections::HashMap};

use rbx_dom_weak::{
    types::{Ref, Variant},
    Instance, WeakDom,
};
use serde::{Deserialize, Serialize};

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
    pub name: Cow<'static, str>,

    /// Corresponds to the ClassName property of the instance.
    pub class_name: Cow<'static, str>,

    /// All other properties of the instance, weakly-typed.
    pub properties: HashMap<String, Variant>,

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

    pub fn property<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<Variant>,
    {
        self.properties.insert(key.into(), value.into());
        self
    }

    pub fn properties(self, properties: impl Into<HashMap<String, Variant>>) -> Self {
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

    #[profiling::function]
    pub fn from_tree(tree: WeakDom, id: Ref) -> Self {
        let (_, mut raw_tree) = tree.into_raw();
        Self::from_raw_tree(&mut raw_tree, id)
    }

    fn from_raw_tree(raw_tree: &mut HashMap<Ref, Instance>, id: Ref) -> Self {
        let instance = raw_tree
            .remove(&id)
            .expect("instance did not exist in tree");

        let children = instance
            .children()
            .iter()
            .map(|&id| Self::from_raw_tree(raw_tree, id))
            .collect();

        Self {
            snapshot_id: Some(id),
            metadata: InstanceMetadata::default(),
            name: Cow::Owned(instance.name),
            class_name: Cow::Owned(instance.class),
            properties: instance.properties,
            children,
        }
    }
}

impl Default for InstanceSnapshot {
    fn default() -> Self {
        Self::new()
    }
}
