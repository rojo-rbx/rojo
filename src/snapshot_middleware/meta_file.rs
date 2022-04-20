use std::{borrow::Cow, collections::HashMap, iter::FromIterator, path::PathBuf};

use anyhow::{format_err, Context};
use serde::{Deserialize, Serialize};

use crate::{resolution::UnresolvedValue, snapshot::InstanceSnapshot};
use rbx_dom_weak::types::{Attributes, Tags, Variant};

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
    pub properties: HashMap<String, UnresolvedValue>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub attributes: HashMap<String, Variant>,

    #[serde(skip)]
    pub path: PathBuf,
}

impl AdjacentMetadata {
    pub fn from_slice(slice: &[u8], path: PathBuf) -> anyhow::Result<Self> {
        let mut meta: Self = serde_json::from_slice(slice).with_context(|| {
            format!(
                "File contained malformed .meta.json data: {}",
                path.display()
            )
        })?;

        meta.path = path;
        Ok(meta)
    }

    pub fn apply_ignore_unknown_instances(&mut self, snapshot: &mut InstanceSnapshot) {
        if let Some(ignore) = self.ignore_unknown_instances.take() {
            snapshot.metadata.ignore_unknown_instances = ignore;
        }
    }

    pub fn apply_properties(&mut self, snapshot: &mut InstanceSnapshot) -> anyhow::Result<()> {
        let path = &self.path;

        for (key, unresolved) in self.properties.drain() {
            let value = unresolved
                .resolve(&snapshot.class_name, &key)
                .with_context(|| format!("error applying meta file {}", path.display()))?;

            snapshot.properties.insert(key, value);
        }

        Ok(())
    }

    fn apply_tags(&mut self, snapshot: &mut InstanceSnapshot) -> anyhow::Result<()> {
        if !self.tags.is_empty() {
            let tags = Tags::from(self.tags.clone());
            snapshot.properties.insert("Tags".into(), tags.into());
        }

        Ok(())
    }

    fn apply_attributes(&mut self, snapshot: &mut InstanceSnapshot) -> anyhow::Result<()> {
        if !self.attributes.is_empty() {
            let attributes = Attributes::from_iter(self.attributes.clone().into_iter());
            snapshot
                .properties
                .insert("Attributes".into(), attributes.into());
        }

        Ok(())
    }

    pub fn apply_all(&mut self, snapshot: &mut InstanceSnapshot) -> anyhow::Result<()> {
        self.apply_ignore_unknown_instances(snapshot);
        self.apply_properties(snapshot)?;
        self.apply_attributes(snapshot)?;
        self.apply_tags(snapshot)?;
        Ok(())
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
    pub properties: HashMap<String, UnresolvedValue>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub attributes: HashMap<String, Variant>,

    #[serde(skip)]
    pub path: PathBuf,
}

impl DirectoryMetadata {
    pub fn from_slice(slice: &[u8], path: PathBuf) -> anyhow::Result<Self> {
        let mut meta: Self = serde_json::from_slice(slice).with_context(|| {
            format!(
                "File contained malformed init.meta.json data: {}",
                path.display()
            )
        })?;

        meta.path = path;
        Ok(meta)
    }

    pub fn apply_all(&mut self, snapshot: &mut InstanceSnapshot) -> anyhow::Result<()> {
        self.apply_ignore_unknown_instances(snapshot);
        self.apply_class_name(snapshot)?;
        self.apply_properties(snapshot)?;
        self.apply_attributes(snapshot)?;
        self.apply_tags(snapshot)?;

        Ok(())
    }

    fn apply_class_name(&mut self, snapshot: &mut InstanceSnapshot) -> anyhow::Result<()> {
        if let Some(class_name) = self.class_name.take() {
            if snapshot.class_name != "Folder" {
                // TODO: Turn into error type
                return Err(format_err!(
                    "className in init.meta.json can only be specified if the \
                     affected directory would turn into a Folder instance."
                ));
            }

            snapshot.class_name = Cow::Owned(class_name);
        }

        Ok(())
    }

    fn apply_ignore_unknown_instances(&mut self, snapshot: &mut InstanceSnapshot) {
        if let Some(ignore) = self.ignore_unknown_instances.take() {
            snapshot.metadata.ignore_unknown_instances = ignore;
        }
    }

    fn apply_properties(&mut self, snapshot: &mut InstanceSnapshot) -> anyhow::Result<()> {
        let path = &self.path;

        for (key, unresolved) in self.properties.drain() {
            let value = unresolved
                .resolve(&snapshot.class_name, &key)
                .with_context(|| format!("error applying meta file {}", path.display()))?;

            snapshot.properties.insert(key, value);
        }

        Ok(())
    }

    fn apply_tags(&mut self, snapshot: &mut InstanceSnapshot) -> anyhow::Result<()> {
        if !self.tags.is_empty() {
            let tags = Tags::from(self.tags.clone());
            snapshot.properties.insert("Tags".into(), tags.into());
        }

        Ok(())
    }

    fn apply_attributes(&mut self, snapshot: &mut InstanceSnapshot) -> anyhow::Result<()> {
        if !self.attributes.is_empty() {
            let attributes = Attributes::from_iter(self.attributes.clone().into_iter());
            snapshot
                .properties
                .insert("Attributes".into(), attributes.into());
        }

        Ok(())
    }
}
