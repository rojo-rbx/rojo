use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use rbx_dom_weak::{Descendants, RbxId, RbxInstance, RbxInstanceProperties, RbxTree, RbxValue};

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
            inner: RbxTree::new(root.properties),
            metadata_map: HashMap::new(),
            path_to_ids: MultiMap::new(),
        };

        tree.insert_metadata(tree.inner.get_root_id(), root.metadata);
        tree
    }

    pub fn inner(&self) -> &RbxTree {
        &self.inner
    }

    pub fn get_root_id(&self) -> RbxId {
        self.inner.get_root_id()
    }

    pub fn get_instance(&self, id: RbxId) -> Option<InstanceWithMeta> {
        if let Some(instance) = self.inner.get_instance(id) {
            let metadata = self.metadata_map.get(&id).unwrap();

            Some(InstanceWithMeta { instance, metadata })
        } else {
            None
        }
    }

    pub fn get_instance_mut(&mut self, id: RbxId) -> Option<InstanceWithMetaMut> {
        if let Some(instance) = self.inner.get_instance_mut(id) {
            let metadata = self.metadata_map.get_mut(&id).unwrap();

            Some(InstanceWithMetaMut { instance, metadata })
        } else {
            None
        }
    }

    pub fn insert_instance(
        &mut self,
        properties: InstancePropertiesWithMeta,
        parent_id: RbxId,
    ) -> RbxId {
        let id = self.inner.insert_instance(properties.properties, parent_id);
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

    /// Replaces the metadata associated with the given instance ID.
    pub fn update_metadata(&mut self, id: RbxId, metadata: InstanceMetadata) {
        use std::collections::hash_map::Entry;

        match self.metadata_map.entry(id) {
            Entry::Occupied(mut entry) => {
                let existing_metadata = entry.get();

                // If this instance's source path changed, we need to update our
                // path associations so that file changes will trigger updates
                // to this instance correctly.
                if existing_metadata.relevant_paths != metadata.relevant_paths {
                    for existing_path in &existing_metadata.relevant_paths {
                        self.path_to_ids.remove(existing_path, id);
                    }

                    for new_path in &metadata.relevant_paths {
                        self.path_to_ids.insert(new_path.clone(), id);
                    }
                }

                entry.insert(metadata);
            }
            Entry::Vacant(entry) => {
                entry.insert(metadata);
            }
        }
    }

    pub fn descendants(&self, id: RbxId) -> RojoDescendants<'_> {
        RojoDescendants {
            inner: self.inner.descendants(id),
            tree: self,
        }
    }

    pub fn get_ids_at_path(&self, path: &Path) -> &[RbxId] {
        self.path_to_ids.get(path)
    }

    pub fn get_metadata(&self, id: RbxId) -> Option<&InstanceMetadata> {
        self.metadata_map.get(&id)
    }

    fn insert_metadata(&mut self, id: RbxId, metadata: InstanceMetadata) {
        for path in &metadata.relevant_paths {
            self.path_to_ids.insert(path.clone(), id);
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

        for path in &metadata.relevant_paths {
            self.path_to_ids.remove(path, id);
            path_to_ids.insert(path.clone(), id);
        }

        metadata_map.insert(id, metadata);
    }
}

pub struct RojoDescendants<'a> {
    inner: Descendants<'a>,
    tree: &'a RojoTree,
}

impl<'a> Iterator for RojoDescendants<'a> {
    type Item = InstanceWithMeta<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let instance = self.inner.next()?;
        let metadata = self
            .tree
            .get_metadata(instance.get_id())
            .expect("Metadata did not exist for instance");

        Some(InstanceWithMeta { instance, metadata })
    }
}

/// RojoTree's equivalent of `RbxInstanceProperties`.
#[derive(Debug, Clone)]
pub struct InstancePropertiesWithMeta {
    pub properties: RbxInstanceProperties,
    pub metadata: InstanceMetadata,
}

impl InstancePropertiesWithMeta {
    pub fn new(properties: RbxInstanceProperties, metadata: InstanceMetadata) -> Self {
        InstancePropertiesWithMeta {
            properties,
            metadata,
        }
    }
}

/// RojoTree's equivalent of `&'a RbxInstance`.
///
/// This has to be a value type for RojoTree because the instance and metadata
/// are stored in different places. The mutable equivalent is
/// `InstanceWithMetaMut`.
#[derive(Debug, Clone, Copy)]
pub struct InstanceWithMeta<'a> {
    instance: &'a RbxInstance,
    metadata: &'a InstanceMetadata,
}

impl<'a> InstanceWithMeta<'a> {
    pub fn id(&self) -> RbxId {
        self.instance.get_id()
    }

    pub fn parent(&self) -> Option<RbxId> {
        self.instance.get_parent_id()
    }

    pub fn name(&self) -> &'a str {
        &self.instance.name
    }

    pub fn class_name(&self) -> &'a str {
        &self.instance.class_name
    }

    pub fn properties(&self) -> &'a HashMap<String, RbxValue> {
        &self.instance.properties
    }

    pub fn children(&self) -> &'a [RbxId] {
        self.instance.get_children_ids()
    }

    pub fn metadata(&self) -> &'a InstanceMetadata {
        &self.metadata
    }
}

/// RojoTree's equivalent of `&'a mut RbxInstance`.
///
/// This has to be a value type for RojoTree because the instance and metadata
/// are stored in different places. The immutable equivalent is
/// `InstanceWithMeta`.
#[derive(Debug)]
pub struct InstanceWithMetaMut<'a> {
    instance: &'a mut RbxInstance,
    metadata: &'a mut InstanceMetadata,
}

impl InstanceWithMetaMut<'_> {
    pub fn id(&self) -> RbxId {
        self.instance.get_id()
    }

    pub fn name(&self) -> &str {
        &self.instance.name
    }

    pub fn name_mut(&mut self) -> &mut String {
        &mut self.instance.name
    }

    pub fn class_name(&self) -> &str {
        &self.instance.class_name
    }

    pub fn class_name_mut(&mut self) -> &mut String {
        &mut self.instance.class_name
    }

    pub fn properties(&self) -> &HashMap<String, RbxValue> {
        &self.instance.properties
    }

    pub fn properties_mut(&mut self) -> &mut HashMap<String, RbxValue> {
        &mut self.instance.properties
    }

    pub fn children(&self) -> &[RbxId] {
        self.instance.get_children_ids()
    }

    pub fn metadata(&self) -> &InstanceMetadata {
        &self.metadata
    }
}
