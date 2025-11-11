use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::{format_err, Context};
use memofs::{IoResultExt as _, Vfs};
use rbx_dom_weak::{types::Attributes, Ustr, UstrMap};
use serde::{Deserialize, Serialize};

use crate::{json, resolution::UnresolvedValue, snapshot::InstanceSnapshot, RojoRef};

/// Represents metadata in a sibling file with the same basename.
///
/// As an example, hello.meta.json next to hello.lua would allow assigning
/// additional metadata to the instance resulting from hello.lua.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdjacentMetadata {
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    schema: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_unknown_instances: Option<bool>,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub properties: UstrMap<UnresolvedValue>,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub attributes: HashMap<String, UnresolvedValue>,

    #[serde(skip)]
    pub path: PathBuf,
}

impl AdjacentMetadata {
    /// Attempts to read a meta file for the provided path and name, and if
    /// one exists applies it.
    ///
    /// Also inserts the potential metadata paths into the snapshot's relevant
    /// paths for convenience purposes.
    pub fn read_and_apply_all(
        vfs: &Vfs,
        path: &Path,
        name: &str,
        snapshot: &mut InstanceSnapshot,
    ) -> anyhow::Result<()> {
        let meta_path_json = path.with_file_name(format!("{name}.meta.json"));
        let meta_path_jsonc = path.with_file_name(format!("{name}.meta.jsonc"));

        if let Some(meta_contents) = vfs.read(&meta_path_json).with_not_found()? {
            let mut metadata = Self::from_slice(&meta_contents, meta_path_json.clone())?;
            metadata.apply_all(snapshot)?;
        }

        if let Some(meta_contents) = vfs.read(&meta_path_jsonc).with_not_found()? {
            let mut metadata = Self::from_slice(&meta_contents, meta_path_json.clone())?;
            metadata.apply_all(snapshot)?;
        }

        // Rather than pushing these in the snapshot middleware, we can just do it here.
        snapshot.metadata.relevant_paths.push(meta_path_json);
        snapshot.metadata.relevant_paths.push(meta_path_jsonc);

        Ok(())
    }

    pub fn from_slice(slice: &[u8], path: PathBuf) -> anyhow::Result<Self> {
        let mut meta: Self = json::from_slice_with_context(slice, || {
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

        if !self.attributes.is_empty() {
            let mut attributes = Attributes::new();

            for (key, unresolved) in self.attributes.drain() {
                let value = unresolved.resolve_unambiguous()?;
                attributes.insert(key, value);
            }

            snapshot
                .properties
                .insert("Attributes".into(), attributes.into());
        }

        Ok(())
    }

    fn apply_id(&mut self, snapshot: &mut InstanceSnapshot) -> anyhow::Result<()> {
        if self.id.is_some() && snapshot.metadata.specified_id.is_some() {
            anyhow::bail!(
                "cannot specify an ID using {} (instance has an ID from somewhere else)",
                self.path.display()
            );
        }
        snapshot.metadata.specified_id = self.id.take().map(RojoRef::new);
        Ok(())
    }

    pub fn apply_all(&mut self, snapshot: &mut InstanceSnapshot) -> anyhow::Result<()> {
        self.apply_ignore_unknown_instances(snapshot);
        self.apply_properties(snapshot)?;
        self.apply_id(snapshot)?;
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
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    schema: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_unknown_instances: Option<bool>,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub properties: UstrMap<UnresolvedValue>,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub attributes: HashMap<String, UnresolvedValue>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_name: Option<Ustr>,

    #[serde(skip)]
    pub path: PathBuf,
}

impl DirectoryMetadata {
    pub fn from_slice(slice: &[u8], path: PathBuf) -> anyhow::Result<Self> {
        let mut meta: Self = json::from_slice_with_context(slice, || {
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
        self.apply_id(snapshot)?;

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

            snapshot.class_name = class_name;
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

        if !self.attributes.is_empty() {
            let mut attributes = Attributes::new();

            for (key, unresolved) in self.attributes.drain() {
                let value = unresolved.resolve_unambiguous()?;
                attributes.insert(key, value);
            }

            snapshot
                .properties
                .insert("Attributes".into(), attributes.into());
        }

        Ok(())
    }

    fn apply_id(&mut self, snapshot: &mut InstanceSnapshot) -> anyhow::Result<()> {
        if self.id.is_some() && snapshot.metadata.specified_id.is_some() {
            anyhow::bail!(
                "cannot specify an ID using {} (instance has an ID from somewhere else)",
                self.path.display()
            );
        }
        snapshot.metadata.specified_id = self.id.take().map(RojoRef::new);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use memofs::{InMemoryFs, VfsSnapshot};

    use super::*;

    #[test]
    fn adjacent_read_json() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(
            "/foo/bar.meta.json",
            VfsSnapshot::file(r#"{"id": "manually specified"}"#),
        )
        .unwrap();

        let vfs = Vfs::new(imfs);
        let path = Path::new("/foo/bar.rojo");
        let mut snapshot = InstanceSnapshot::new();

        AdjacentMetadata::read_and_apply_all(&vfs, path, "bar", &mut snapshot).unwrap();

        insta::assert_yaml_snapshot!(snapshot);
    }

    #[test]
    fn adjacent_read_jsonc() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(
            "/foo/bar.meta.jsonc",
            VfsSnapshot::file(r#"{"id": "manually specified"}"#),
        )
        .unwrap();

        let vfs = Vfs::new(imfs);
        let path = Path::new("/foo/bar.rojo");
        let mut snapshot = InstanceSnapshot::new();

        AdjacentMetadata::read_and_apply_all(&vfs, path, "bar", &mut snapshot).unwrap();

        insta::assert_yaml_snapshot!(snapshot);
    }
}
