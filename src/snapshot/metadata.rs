use std::{collections::HashMap, path::PathBuf};

use rbx_dom_weak::{RbxId, RbxInstance, RbxInstanceProperties, RbxTree};

use crate::project::ProjectNode;

/// Rojo-specific metadata that can be associated with an instance or a snapshot
/// of an instance.
#[derive(Debug, Clone, PartialEq)]
pub struct InstanceMetadata {
    /// Whether instances not present in the source should be ignored when
    /// live-syncing. This is useful when there are instances that Rojo does not
    /// manage.
    pub ignore_unknown_instances: bool,

    /// A complete view of where this snapshot came from. It should contain
    /// enough information, if not None, to recreate this snapshot
    /// deterministically assuming the source has not changed state.
    pub source: Option<InstanceSource>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InstanceSource {
    File {
        path: PathBuf,
    },
    ProjectFile {
        path: PathBuf,
        name: String,
        node: ProjectNode,
    },
}

impl Default for InstanceMetadata {
    fn default() -> Self {
        InstanceMetadata {
            ignore_unknown_instances: false,
            source: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstancePropertiesWithMeta {
    pub inner: RbxInstanceProperties,
    pub metadata: InstanceMetadata,
}

#[derive(Debug)]
pub struct InstanceWithMeta<'a> {
    pub inner: &'a RbxInstance,
    pub metadata: &'a InstanceMetadata,
}

#[derive(Debug)]
pub struct InstanceWithMetaMut<'a> {
    pub inner: &'a mut RbxInstance,
    pub metadata: &'a mut InstanceMetadata,
}

#[derive(Debug)]
pub struct TreeWithMetadata {
    inner: RbxTree,
    metadata: HashMap<RbxId, InstanceMetadata>,
}

impl TreeWithMetadata {
    pub fn new(root: InstancePropertiesWithMeta) -> TreeWithMetadata {
        let inner = RbxTree::new(root.inner);
        let mut metadata = HashMap::new();
        metadata.insert(inner.get_root_id(), root.metadata);

        TreeWithMetadata { inner, metadata }
    }

    pub fn get_root_id(&self) -> RbxId {
        self.inner.get_root_id()
    }

    pub fn get_instance(&self, id: RbxId) -> Option<InstanceWithMeta> {
        if let Some(inner) = self.inner.get_instance(id) {
            let metadata = self.metadata.get(&id).unwrap();

            Some(InstanceWithMeta { inner, metadata })
        } else {
            None
        }
    }

    pub fn get_instance_mut(&mut self, id: RbxId) -> Option<InstanceWithMetaMut> {
        if let Some(inner) = self.inner.get_instance_mut(id) {
            let metadata = self.metadata.get_mut(&id).unwrap();

            Some(InstanceWithMetaMut { inner, metadata })
        } else {
            None
        }
    }

    pub fn insert_instance(
        &mut self,
        properties: InstancePropertiesWithMeta,
        parent_id: RbxId,
    ) -> RbxId {
        let id = self.inner.insert_instance(properties.inner, parent_id);
        self.metadata.insert(id, properties.metadata);
        id
    }

    pub fn remove_instance(&mut self, id: RbxId) -> Option<TreeWithMetadata> {
        if let Some(inner) = self.inner.remove_instance(id) {
            let mut metadata = HashMap::new();

            let root_meta = self.metadata.remove(&id).unwrap();
            metadata.insert(id, root_meta);

            for instance in inner.descendants(id) {
                let instance_meta = self.metadata.remove(&instance.get_id()).unwrap();
                metadata.insert(instance.get_id(), instance_meta);
            }

            Some(TreeWithMetadata { inner, metadata })
        } else {
            None
        }
    }
}
