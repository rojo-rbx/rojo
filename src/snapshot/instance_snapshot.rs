//! Defines the structure of an instance snapshot.

use std::{borrow::Cow, collections::HashMap, path::PathBuf};

use rbx_dom_weak::{RbxId, RbxTree, RbxValue};

use crate::project::ProjectNode;

/// A lightweight description of what an instance should look like. Attempts to
/// be somewhat memory efficient by borrowing from its source data, indicated by
/// the lifetime parameter `'source`.
///
// Possible future improvements:
// - Use refcounted/interned strings
// - Replace use of RbxValue with a sum of RbxValue + borrowed value
#[derive(Debug, Clone, PartialEq)]
pub struct InstanceSnapshot<'source> {
    /// A temporary ID applied to the snapshot that's used for Ref properties.
    pub snapshot_id: Option<RbxId>,

    /// A complete view of where this snapshot came from. It should contain
    /// enough information, if not None, to recreate this snapshot
    /// deterministically assuming the source has not changed state.
    pub source: Option<SnapshotSource>,

    pub name: Cow<'source, str>,
    pub class_name: Cow<'source, str>,
    pub properties: HashMap<String, RbxValue>,
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
            source: self.source.clone(),
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
            source: None,
            name: Cow::Owned(instance.name.clone()),
            class_name: Cow::Owned(instance.class_name.clone()),
            properties: instance.properties.clone(),
            children,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SnapshotSource {
    File {
        path: PathBuf,
    },
    ProjectFile {
        path: PathBuf,
        name: String,
        node: ProjectNode,
    },
}
