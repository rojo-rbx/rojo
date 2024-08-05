use std::{
    borrow::Cow,
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use anyhow::{format_err, Context};
use memofs::{IoResultExt as _, Vfs};
use rbx_dom_weak::types::{Attributes, Variant};
use serde::{Deserialize, Serialize};

use crate::{
    resolution::UnresolvedValue, snapshot::InstanceSnapshot, syncback::SyncbackSnapshot, RojoRef,
};

/// Represents metadata in a sibling file with the same basename.
///
/// As an example, hello.meta.json next to hello.lua would allow assigning
/// additional metadata to the instance resulting from hello.lua.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdjacentMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_unknown_instances: Option<bool>,

    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub properties: BTreeMap<String, UnresolvedValue>,

    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attributes: BTreeMap<String, UnresolvedValue>,

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

    /// Constructs an `AdjacentMetadata` from the provided snapshot, assuming it
    /// will be at the provided path.
    pub fn from_syncback_snapshot(
        snapshot: &SyncbackSnapshot,
        path: PathBuf,
    ) -> anyhow::Result<Option<Self>> {
        let mut properties = BTreeMap::new();
        let mut attributes = BTreeMap::new();
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

        let class = &snapshot.new_inst().class;
        for (name, value) in snapshot.get_path_filtered_properties(snapshot.new).unwrap() {
            match value {
                Variant::Attributes(attrs) => {
                    for (attr_name, attr_value) in attrs.iter() {
                        attributes.insert(
                            attr_name.clone(),
                            UnresolvedValue::from_variant_unambiguous(attr_value.clone()),
                        );
                    }
                }
                Variant::SharedString(_) => {
                    log::warn!(
                    "Rojo cannot serialize the property {}.{name} in meta.json files.\n\
                    If this is not acceptable, resave the Instance at '{}' manually as an RBXM or RBXMX.", class, snapshot.get_new_inst_path(snapshot.new))
                }
                _ => {
                    properties.insert(
                        name.to_owned(),
                        UnresolvedValue::from_variant(value.clone(), class, name),
                    );
                }
            }
        }

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

    pub fn apply_all(&mut self, snapshot: &mut InstanceSnapshot) -> anyhow::Result<()> {
        self.apply_ignore_unknown_instances(snapshot);
        self.apply_properties(snapshot)?;
        self.apply_id(snapshot)?;
        Ok(())
    }

    /// Returns whether the metadata is 'empty', meaning it doesn't have anything
    /// worth persisting in it. Specifically:
    ///
    /// - The number of properties and attributes is 0
    /// - `ignore_unknown_instances` is None
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.attributes.is_empty()
            && self.properties.is_empty()
            && self.ignore_unknown_instances.is_none()
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
    pub id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_unknown_instances: Option<bool>,

    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub properties: BTreeMap<String, UnresolvedValue>,

    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attributes: BTreeMap<String, UnresolvedValue>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,

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

    /// Constructs a `DirectoryMetadata` from the provided snapshot, assuming it
    /// will be at the provided path.
    ///
    /// This function does not set `ClassName` manually as most uses won't
    /// want it set.
    pub fn from_syncback_snapshot(
        snapshot: &SyncbackSnapshot,
        path: PathBuf,
    ) -> anyhow::Result<Option<Self>> {
        let mut properties = BTreeMap::new();
        let mut attributes = BTreeMap::new();
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

        let class = &snapshot.new_inst().class;
        for (name, value) in snapshot.get_path_filtered_properties(snapshot.new).unwrap() {
            match value {
                Variant::Attributes(attrs) => {
                    for (name, value) in attrs.iter() {
                        attributes.insert(
                            name.to_owned(),
                            UnresolvedValue::from_variant_unambiguous(value.clone()),
                        );
                    }
                }
                Variant::SharedString(_) => {
                    log::warn!(
                    "Rojo cannot serialize the property {}.{name} in meta.json files.\n\
                    If this is not acceptable, resave the Instance at '{}' manually as an RBXM or RBXMX.", class, snapshot.get_new_inst_path(snapshot.new))
                }
                _ => {
                    properties.insert(
                        name.to_owned(),
                        UnresolvedValue::from_variant(value.clone(), class, name),
                    );
                }
            }
        }

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
        }))
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

    /// Returns whether the metadata is 'empty', meaning it doesn't have anything
    /// worth persisting in it. Specifically:
    ///
    /// - The number of properties and attributes is 0
    /// - `ignore_unknown_instances` is None
    /// - `class_name` is either None or not Some("Folder")
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.attributes.is_empty()
            && self.properties.is_empty()
            && self.ignore_unknown_instances.is_none()
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
