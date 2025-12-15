use std::path::{Path, PathBuf};

use anyhow::{format_err, Context};
use indexmap::IndexMap;
use memofs::{IoResultExt as _, Vfs};
use rbx_dom_weak::{
    types::{Attributes, Variant},
    Ustr,
};
use serde::{Deserialize, Serialize};

use crate::{
    json,
    resolution::UnresolvedValue,
    snapshot::InstanceSnapshot,
    syncback::{validate_file_name, SyncbackSnapshot},
    RojoRef,
};

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

    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub properties: IndexMap<Ustr, UnresolvedValue>,

    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub attributes: IndexMap<String, UnresolvedValue>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

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

    fn from_slice(slice: &[u8], path: PathBuf) -> anyhow::Result<Self> {
        let mut meta: Self = json::from_slice_with_context(slice, || {
            format!(
                "File contained malformed .meta.json data: {}",
                path.display()
            )
        })?;

        meta.path = path;
        Ok(meta)
    }

    /// Constructs an `AdjacentMetadata` from the provided snapshot, assuming it
    /// will be at the provided path.
    pub fn from_syncback_snapshot(
        snapshot: &SyncbackSnapshot,
        path: PathBuf,
    ) -> anyhow::Result<Option<Self>> {
        let mut properties = IndexMap::new();
        let mut attributes = IndexMap::new();
        // TODO make this more granular.
        // I am breaking the cycle of bad TODOs. This is in reference to the fact
        // that right now, this will just not write any metadata at all for
        // project nodes, which is not always desirable. We should try to be
        // smarter about it.
        if let Some(old_inst) = snapshot.old_inst() {
            if let Some(source) = &old_inst.metadata().instigating_source {
                let source = source.path();
                if source != path {
                    log::debug!(
                        "Instigating source for Instance is mismatched so its metadata is being skipped.\nPath: {}",
                        path.display()
                    );
                    return Ok(None);
                }
            }
        }

        let ignore_unknown_instances = snapshot
            .old_inst()
            .map(|inst| inst.metadata().ignore_unknown_instances)
            .unwrap_or_default();

        let schema = snapshot
            .old_inst()
            .and_then(|inst| inst.metadata().schema.clone());

        let class = &snapshot.new_inst().class;
        for (name, value) in snapshot.get_path_filtered_properties(snapshot.new).unwrap() {
            match value {
                Variant::Attributes(attrs) => {
                    for (attr_name, attr_value) in attrs.iter() {
                        // We (probably) don't want to preserve internal
                        // attributes, only user defined ones.
                        if attr_name.starts_with("RBX") {
                            continue;
                        }
                        attributes.insert(
                            attr_name.clone(),
                            UnresolvedValue::from_variant_unambiguous(attr_value.clone()),
                        );
                    }
                }
                _ => {
                    properties.insert(
                        name,
                        UnresolvedValue::from_variant(value.clone(), class, &name),
                    );
                }
            }
        }

        let name = snapshot
            .old_inst()
            .and_then(|inst| inst.metadata().specified_name.clone())
            .or_else(|| {
                // If this is a new instance and its name is invalid for the filesystem,
                // we need to specify the name in meta.json so it can be preserved
                if snapshot.old_inst().is_none() {
                    let instance_name = &snapshot.new_inst().name;
                    if validate_file_name(instance_name).is_err() {
                        Some(instance_name.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            });

        Ok(Some(Self {
            ignore_unknown_instances: if ignore_unknown_instances {
                Some(true)
            } else {
                None
            },
            properties,
            attributes,
            path,
            id: None,
            schema,
            name,
        }))
    }

    pub fn apply_ignore_unknown_instances(&mut self, snapshot: &mut InstanceSnapshot) {
        if let Some(ignore) = self.ignore_unknown_instances.take() {
            snapshot.metadata.ignore_unknown_instances = ignore;
        }
    }

    pub fn apply_properties(&mut self, snapshot: &mut InstanceSnapshot) -> anyhow::Result<()> {
        let path = &self.path;

        // BTreeMaps don't have an equivalent to HashMap::drain, so the next
        // best option is to take ownership of the entire map. Not free, but
        // very cheap.
        for (key, unresolved) in std::mem::take(&mut self.properties) {
            let value = unresolved
                .resolve(&snapshot.class_name, &key)
                .with_context(|| format!("error applying meta file {}", path.display()))?;

            snapshot.properties.insert(key, value);
        }

        if !self.attributes.is_empty() {
            let mut attributes = Attributes::new();

            for (key, unresolved) in std::mem::take(&mut self.attributes) {
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

    fn apply_schema(&mut self, snapshot: &mut InstanceSnapshot) -> anyhow::Result<()> {
        if self.schema.is_some() && snapshot.metadata.schema.is_some() {
            anyhow::bail!("cannot specify a schema using {} (instance has a schema from somewhere else. how did we get here?)", self.path.display());
        }
        snapshot.metadata.schema = self.schema.take();
        Ok(())
    }

    fn apply_name(&mut self, snapshot: &mut InstanceSnapshot) -> anyhow::Result<()> {
        if self.name.is_some() && snapshot.metadata.specified_name.is_some() {
            anyhow::bail!(
                "cannot specify a name using {} (instance has a name from somewhere else)",
                self.path.display()
            );
        }
        snapshot.metadata.specified_name = self.name.take();
        Ok(())
    }

    pub fn apply_all(&mut self, snapshot: &mut InstanceSnapshot) -> anyhow::Result<()> {
        self.apply_ignore_unknown_instances(snapshot);
        self.apply_properties(snapshot)?;
        self.apply_id(snapshot)?;
        self.apply_schema(snapshot)?;
        self.apply_name(snapshot)?;
        Ok(())
    }

    /// Returns whether the metadata is 'empty', meaning it doesn't have anything
    /// worth persisting in it. Specifically:
    ///
    /// - The number of properties and attributes is 0
    /// - `ignore_unknown_instances` is None
    /// - `name` is None
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.attributes.is_empty()
            && self.properties.is_empty()
            && self.ignore_unknown_instances.is_none()
            && self.name.is_none()
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

    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub properties: IndexMap<Ustr, UnresolvedValue>,

    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub attributes: IndexMap<String, UnresolvedValue>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_name: Option<Ustr>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip)]
    pub path: PathBuf,
}

impl DirectoryMetadata {
    /// Attempts to read an `init.meta`` file for the provided path, and if
    /// one exists applies it.
    ///
    /// Also inserts the potential metadata paths into the snapshot's relevant
    /// paths for convenience purposes.
    pub fn read_and_apply_all(
        vfs: &Vfs,
        path: &Path,
        snapshot: &mut InstanceSnapshot,
    ) -> anyhow::Result<()> {
        let meta_path_json = path.join("init.meta.json");
        let meta_path_jsonc = path.join("init.meta.jsonc");

        if let Some(meta_contents) = vfs.read(&meta_path_json).with_not_found()? {
            let mut metadata = Self::from_slice(&meta_contents, meta_path_json.clone())?;
            metadata.apply_all(snapshot)?;
        }

        if let Some(meta_contents) = vfs.read(&meta_path_jsonc).with_not_found()? {
            let mut metadata = Self::from_slice(&meta_contents, meta_path_jsonc.clone())?;
            metadata.apply_all(snapshot)?;
        }

        // Rather than pushing these in the snapshot middleware, we can just do it here.
        snapshot.metadata.relevant_paths.push(meta_path_json);
        snapshot.metadata.relevant_paths.push(meta_path_jsonc);

        Ok(())
    }

    fn from_slice(slice: &[u8], path: PathBuf) -> anyhow::Result<Self> {
        let mut meta: Self = json::from_slice_with_context(slice, || {
            format!(
                "File contained malformed init.meta.json data: {}",
                path.display()
            )
        })?;

        meta.path = path;
        Ok(meta)
    }

    /// Constructs a `DirectoryMetadata` from the provided snapshot, assuming it
    /// will be at the provided path.
    ///
    /// This function does not set `ClassName` manually as most uses won't
    /// want it set.
    pub fn from_syncback_snapshot(
        snapshot: &SyncbackSnapshot,
        path: PathBuf,
    ) -> anyhow::Result<Option<Self>> {
        let mut properties = IndexMap::new();
        let mut attributes = IndexMap::new();
        // TODO make this more granular.
        // I am breaking the cycle of bad TODOs. This is in reference to the fact
        // that right now, this will just not write any metadata at all for
        // project nodes, which is not always desirable. We should try to be
        // smarter about it.
        if let Some(old_inst) = snapshot.old_inst() {
            if let Some(source) = &old_inst.metadata().instigating_source {
                let source = source.path();
                if source != path {
                    log::debug!(
                        "Instigating source for Instance is mismatched so its metadata is being skipped.\nPath: {}",
                        path.display()
                    );
                    return Ok(None);
                }
            }
        }

        let ignore_unknown_instances = snapshot
            .old_inst()
            .map(|inst| inst.metadata().ignore_unknown_instances)
            .unwrap_or_default();

        let schema = snapshot
            .old_inst()
            .and_then(|inst| inst.metadata().schema.clone());

        let class = &snapshot.new_inst().class;
        for (name, value) in snapshot.get_path_filtered_properties(snapshot.new).unwrap() {
            match value {
                Variant::Attributes(attrs) => {
                    for (name, value) in attrs.iter() {
                        // We (probably) don't want to preserve internal
                        // attributes, only user defined ones.
                        if name.starts_with("RBX") {
                            continue;
                        }
                        attributes.insert(
                            name.to_owned(),
                            UnresolvedValue::from_variant_unambiguous(value.clone()),
                        );
                    }
                }
                _ => {
                    properties.insert(
                        name,
                        UnresolvedValue::from_variant(value.clone(), class, &name),
                    );
                }
            }
        }

        let name = snapshot
            .old_inst()
            .and_then(|inst| inst.metadata().specified_name.clone())
            .or_else(|| {
                // If this is a new instance and its name is invalid for the filesystem,
                // we need to specify the name in meta.json so it can be preserved
                if snapshot.old_inst().is_none() {
                    let instance_name = &snapshot.new_inst().name;
                    if validate_file_name(instance_name).is_err() {
                        Some(instance_name.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            });

        Ok(Some(Self {
            ignore_unknown_instances: if ignore_unknown_instances {
                Some(true)
            } else {
                None
            },
            properties,
            attributes,
            class_name: None,
            path,
            id: None,
            schema,
            name,
        }))
    }

    pub fn apply_all(&mut self, snapshot: &mut InstanceSnapshot) -> anyhow::Result<()> {
        self.apply_ignore_unknown_instances(snapshot);
        self.apply_class_name(snapshot)?;
        self.apply_properties(snapshot)?;
        self.apply_id(snapshot)?;
        self.apply_schema(snapshot)?;
        self.apply_name(snapshot)?;

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

        for (key, unresolved) in std::mem::take(&mut self.properties) {
            let value = unresolved
                .resolve(&snapshot.class_name, &key)
                .with_context(|| format!("error applying meta file {}", path.display()))?;

            snapshot.properties.insert(key, value);
        }

        if !self.attributes.is_empty() {
            let mut attributes = Attributes::new();

            for (key, unresolved) in std::mem::take(&mut self.attributes) {
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

    fn apply_schema(&mut self, snapshot: &mut InstanceSnapshot) -> anyhow::Result<()> {
        if self.schema.is_some() && snapshot.metadata.schema.is_some() {
            anyhow::bail!("cannot specify a schema using {} (instance has a schema from somewhere else. how did we get here?)", self.path.display());
        }
        snapshot.metadata.schema = self.schema.take();
        Ok(())
    }

    fn apply_name(&mut self, snapshot: &mut InstanceSnapshot) -> anyhow::Result<()> {
        if self.name.is_some() && snapshot.metadata.specified_name.is_some() {
            anyhow::bail!(
                "cannot specify a name using {} (instance has a name from somewhere else)",
                self.path.display()
            );
        }
        snapshot.metadata.specified_name = self.name.take();
        Ok(())
    }
    /// Returns whether the metadata is 'empty', meaning it doesn't have anything
    /// worth persisting in it. Specifically:
    ///
    /// - The number of properties and attributes is 0
    /// - `ignore_unknown_instances` is None
    /// - `class_name` is either None or not Some("Folder")
    /// - `name` is None
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.attributes.is_empty()
            && self.properties.is_empty()
            && self.ignore_unknown_instances.is_none()
            && self.name.is_none()
            && if let Some(class) = &self.class_name {
                class == "Folder"
            } else {
                true
            }
    }
}

/// Retrieves the meta file that should be applied for the provided directory,
/// if it exists.
pub fn dir_meta(vfs: &Vfs, path: &Path) -> anyhow::Result<Option<DirectoryMetadata>> {
    let meta_path = path.join("init.meta.json");

    if let Some(meta_contents) = vfs.read(&meta_path).with_not_found()? {
        let metadata = DirectoryMetadata::from_slice(&meta_contents, meta_path)?;
        Ok(Some(metadata))
    } else {
        Ok(None)
    }
}

/// Retrieves the meta file that should be applied for the provided file,
/// if it exists.
///
/// The `name` field should be the name the metadata should have.
pub fn file_meta(vfs: &Vfs, path: &Path, name: &str) -> anyhow::Result<Option<AdjacentMetadata>> {
    let mut meta_path = path.with_file_name(name);
    meta_path.set_extension("meta.json");

    if let Some(meta_contents) = vfs.read(&meta_path).with_not_found()? {
        let metadata = AdjacentMetadata::from_slice(&meta_contents, meta_path)?;
        Ok(Some(metadata))
    } else {
        Ok(None)
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

    #[test]
    fn directory_read_json() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(
            "/foo/init.meta.json",
            VfsSnapshot::file(r#"{"id": "manually specified"}"#),
        )
        .unwrap();

        let vfs = Vfs::new(imfs);
        let path = Path::new("/foo/");
        let mut snapshot = InstanceSnapshot::new();

        DirectoryMetadata::read_and_apply_all(&vfs, path, &mut snapshot).unwrap();

        insta::assert_yaml_snapshot!(snapshot);
    }

    #[test]
    fn directory_read_jsonc() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(
            "/foo/init.meta.jsonc",
            VfsSnapshot::file(r#"{"id": "manually specified"}"#),
        )
        .unwrap();

        let vfs = Vfs::new(imfs);
        let path = Path::new("/foo/");
        let mut snapshot = InstanceSnapshot::new();

        DirectoryMetadata::read_and_apply_all(&vfs, path, &mut snapshot).unwrap();

        insta::assert_yaml_snapshot!(snapshot);
    }
}
