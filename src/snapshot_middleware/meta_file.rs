use std::{borrow::Cow, collections::HashMap, path::Path};

use rbx_dom_weak::UnresolvedRbxValue;
use rbx_reflection::try_resolve_value;
use serde::{Deserialize, Serialize};

use crate::snapshot::InstanceSnapshot;

use super::error::SnapshotError;

/// Represents metadata in a sibling file with the same basename.
///
/// As an example, hello.meta.json next to hello.lua would allow assigning
/// additional metadata to the instance resulting from hello.lua.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdjacentMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_unknown_instances: Option<bool>,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub properties: HashMap<String, UnresolvedRbxValue>,
}

impl AdjacentMetadata {
    pub fn from_slice(slice: &[u8], path: &Path) -> Result<Self, SnapshotError> {
        serde_json::from_slice(slice)
            .map_err(|source| SnapshotError::malformed_meta_json(source, path))
    }

    pub fn apply_ignore_unknown_instances(&mut self, snapshot: &mut InstanceSnapshot) {
        if let Some(ignore) = self.ignore_unknown_instances.take() {
            snapshot.metadata.ignore_unknown_instances = ignore;
        }
    }

    pub fn apply_properties(&mut self, snapshot: &mut InstanceSnapshot) {
        let class_name = &snapshot.class_name;

        let source_properties = self.properties.drain().map(|(key, value)| {
            try_resolve_value(class_name, &key, &value)
                .map(|resolved| (key, resolved))
                .expect("TODO: Handle rbx_reflection errors")
        });

        for (key, value) in source_properties {
            snapshot.properties.insert(key, value);
        }
    }

    pub fn apply_all(&mut self, snapshot: &mut InstanceSnapshot) {
        self.apply_ignore_unknown_instances(snapshot);
        self.apply_properties(snapshot);
    }

    // TODO: Add method to allow selectively applying parts of metadata and
    // throwing errors if invalid parts are specified.
}

/// Represents metadata that affects the instance resulting from the containing
/// folder.
///
/// This is always sourced from a file named init.meta.json.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_unknown_instances: Option<bool>,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub properties: HashMap<String, UnresolvedRbxValue>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,
}

impl DirectoryMetadata {
    pub fn from_slice(slice: &[u8], path: &Path) -> Result<Self, SnapshotError> {
        serde_json::from_slice(slice)
            .map_err(|source| SnapshotError::malformed_meta_json(source, path))
    }

    pub fn apply_all(&mut self, snapshot: &mut InstanceSnapshot) {
        self.apply_ignore_unknown_instances(snapshot);
        self.apply_class_name(snapshot);
        self.apply_properties(snapshot);
    }

    fn apply_class_name(&mut self, snapshot: &mut InstanceSnapshot) {
        if let Some(class_name) = self.class_name.take() {
            if snapshot.class_name != "Folder" {
                // TODO: Turn into error type
                panic!("className in init.meta.json can only be specified if the affected directory would turn into a Folder instance.");
            }

            snapshot.class_name = Cow::Owned(class_name);
        }
    }

    fn apply_ignore_unknown_instances(&mut self, snapshot: &mut InstanceSnapshot) {
        if let Some(ignore) = self.ignore_unknown_instances.take() {
            snapshot.metadata.ignore_unknown_instances = ignore;
        }
    }

    fn apply_properties(&mut self, snapshot: &mut InstanceSnapshot) {
        let class_name = &snapshot.class_name;

        let source_properties = self.properties.drain().map(|(key, value)| {
            try_resolve_value(class_name, &key, &value)
                .map(|resolved| (key, resolved))
                .expect("TODO: Handle rbx_reflection errors")
        });

        for (key, value) in source_properties {
            snapshot.properties.insert(key, value);
        }
    }
}
