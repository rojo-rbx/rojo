use std::{collections::HashMap, path::PathBuf};

use rbx_dom_weak::{RbxId, RbxInstance, RbxInstanceProperties, RbxTree};

use crate::mapset::MapSet;

use super::InstanceMetadata;

/// An expanded variant of rbx_dom_weak's `RbxTree` that tracks additional
/// metadata per instance that's Rojo-specific.
///
/// This tree is also optimized for doing fast incremental updates and patches.
#[derive(Debug)]
pub struct RojoTree {
    /// Contains the instances without their Rojo-specific metadata.
    inner: RbxTree,

    /// Metadata associated with each instance that is kept up-to-date with the
    /// set of actual instances.
    metadata: HashMap<RbxId, InstanceMetadata>,

    /// A multimap from source paths to all of the root instances that were
    /// constructed from that path.
    ///
    /// Descendants of those instances should not be contained in the set, the
    /// value portion of the map is also a set in order to support the same path
    /// appearing multiple times in the same Rojo project. This is sometimes
    /// called "path aliasing" in various Rojo documentation.
    path_to_id: MapSet<PathBuf, RbxId>,
}

impl RojoTree {
    pub fn new(root: InstancePropertiesWithMeta) -> RojoTree {
        let inner = RbxTree::new(root.inner);
        let mut metadata = HashMap::new();
        metadata.insert(inner.get_root_id(), root.metadata);

        RojoTree {
            inner,
            metadata,
            path_to_id: MapSet::new(),
        }
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

    pub fn remove_instance(&mut self, id: RbxId) -> Option<RojoTree> {
        if let Some(inner) = self.inner.remove_instance(id) {
            let mut metadata = HashMap::new();
            let mut path_to_id = MapSet::new(); // TODO

            let root_meta = self.metadata.remove(&id).unwrap();

            metadata.insert(id, root_meta);

            for instance in inner.descendants(id) {
                let instance_meta = self.metadata.remove(&instance.get_id()).unwrap();
                metadata.insert(instance.get_id(), instance_meta);
            }

            Some(RojoTree {
                inner,
                metadata,
                path_to_id,
            })
        } else {
            None
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
