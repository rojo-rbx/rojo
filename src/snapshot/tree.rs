use std::{collections::HashMap, path::PathBuf};

use rbx_dom_weak::{RbxId, RbxInstance, RbxInstanceProperties, RbxTree};

use crate::multimap::MultiMap;

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
    metadata_map: HashMap<RbxId, InstanceMetadata>,

    /// A multimap from source paths to all of the root instances that were
    /// constructed from that path.
    ///
    /// Descendants of those instances should not be contained in the set, the
    /// value portion of the map is also a set in order to support the same path
    /// appearing multiple times in the same Rojo project. This is sometimes
    /// called "path aliasing" in various Rojo documentation.
    path_to_ids: MultiMap<PathBuf, RbxId>,
}

impl RojoTree {
    pub fn new(root: InstancePropertiesWithMeta) -> RojoTree {
        let mut tree = RojoTree {
            inner: RbxTree::new(root.inner),
            metadata_map: HashMap::new(),
            path_to_ids: MultiMap::new(),
        };

        tree.insert_metadata(tree.inner.get_root_id(), root.metadata);
        tree
    }

    pub fn get_root_id(&self) -> RbxId {
        self.inner.get_root_id()
    }

    pub fn get_instance(&self, id: RbxId) -> Option<InstanceWithMeta> {
        if let Some(inner) = self.inner.get_instance(id) {
            let metadata = self.metadata_map.get(&id).unwrap();

            Some(InstanceWithMeta { inner, metadata })
        } else {
            None
        }
    }

    pub fn get_instance_mut(&mut self, id: RbxId) -> Option<InstanceWithMetaMut> {
        if let Some(inner) = self.inner.get_instance_mut(id) {
            let metadata = self.metadata_map.get_mut(&id).unwrap();

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
        self.insert_metadata(id, properties.metadata);
        id
    }

    pub fn remove_instance(&mut self, id: RbxId) -> Option<RojoTree> {
        if let Some(inner) = self.inner.remove_instance(id) {
            let mut metadata_map = HashMap::new();
            let mut path_to_ids = MultiMap::new();

            self.move_metadata(id, &mut metadata_map, &mut path_to_ids);
            for instance in inner.descendants(id) {
                self.move_metadata(instance.get_id(), &mut metadata_map, &mut path_to_ids);
            }

            Some(RojoTree {
                inner,
                metadata_map,
                path_to_ids,
            })
        } else {
            None
        }
    }

    fn insert_metadata(&mut self, id: RbxId, metadata: InstanceMetadata) {
        if let Some(source) = &metadata.source {
            self.path_to_ids.insert(source.path.clone(), id);
        }

        self.metadata_map.insert(id, metadata);
    }

    /// Moves the Rojo metadata from the instance with the given ID from this
    /// tree into some loose maps.
    fn move_metadata(
        &mut self,
        id: RbxId,
        metadata_map: &mut HashMap<RbxId, InstanceMetadata>,
        path_to_ids: &mut MultiMap<PathBuf, RbxId>,
    ) {
        let metadata = self.metadata_map.remove(&id).unwrap();

        if let Some(source) = &metadata.source {
            self.path_to_ids.remove(&source.path, id);
            path_to_ids.insert(source.path.clone(), id);
        }

        metadata_map.insert(id, metadata);
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
