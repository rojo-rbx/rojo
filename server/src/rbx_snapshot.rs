//! Defines how Rojo transforms files into instances through the snapshot
//! system.

use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
    fmt,
    path::{Path, PathBuf},
    str,
};

use rlua::Lua;
use failure::Fail;
use log::info;
use maplit::hashmap;
use rbx_dom_weak::{RbxTree, RbxValue, RbxInstanceProperties, UnresolvedRbxValue};
use serde::{Serialize, Deserialize};
use rbx_reflection::{try_resolve_value, ValueResolveError};

use crate::{
    imfs::{
        Imfs,
        ImfsItem,
        ImfsFile,
        ImfsDirectory,
    },
    project::{
        Project,
        ProjectNode,
    },
    snapshot_reconciler::{
        RbxSnapshotInstance,
        snapshot_from_tree,
    },
    // TODO: Move MetadataPerInstance into this module?
    rbx_session::MetadataPerInstance,
};

const INIT_MODULE_NAME: &str = "init.lua";
const INIT_SERVER_NAME: &str = "init.server.lua";
const INIT_CLIENT_NAME: &str = "init.client.lua";

pub struct SnapshotContext {
    pub plugin_context: Option<SnapshotPluginContext>,
}

/// Context that's only relevant to generating snapshots if there are plugins
/// associated with the project.
///
/// It's possible that this needs some sort of extra nesting/filtering to
/// support nested projects, since their plugins should only apply to
/// themselves.
pub struct SnapshotPluginContext {
    pub lua: Lua,
    pub plugins: Vec<SnapshotPluginEntry>,
}

pub struct SnapshotPluginEntry {
    /// Simple file name suffix filter to avoid running plugins on every file
    /// change.
    pub file_name_filter: String,

    /// A key into the Lua registry created by [`create_registry_value`] that
    /// refers to a function that can be called to transform a file/instance
    /// pair according to how the plugin needs to operate.
    ///
    /// [`create_registry_value`]: https://docs.rs/rlua/0.16.2/rlua/struct.Context.html#method.create_registry_value
    pub callback: rlua::RegistryKey,
}

#[derive(Debug, Clone)]
struct LuaRbxSnapshot(RbxSnapshotInstance<'static>);

impl rlua::UserData for LuaRbxSnapshot {
    fn add_methods<'lua, M: rlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(rlua::MetaMethod::Index, |_context, this, key: String| {
            match key.as_str() {
                "name" => Ok(this.0.name.clone().into_owned()),
                "className" => Ok(this.0.class_name.clone().into_owned()),
                _ => Err(rlua::Error::RuntimeError(format!("{} is not a valid member of RbxSnapshotInstance", &key))),
            }
        });

        methods.add_meta_method(rlua::MetaMethod::ToString, |_context, _this, _args: ()| {
            Ok("RbxSnapshotInstance")
        });
    }
}

pub type SnapshotResult<'a> = Result<Option<RbxSnapshotInstance<'a>>, SnapshotError>;

#[derive(Debug, Fail)]
pub enum SnapshotError {
    DidNotExist(PathBuf),

    Utf8Error {
        #[fail(cause)]
        inner: str::Utf8Error,
        path: PathBuf,
    },

    JsonModelDecodeError {
        #[fail(cause)]
        inner: serde_json::Error,
        path: PathBuf,
    },

    ExtraMetadataError {
        #[fail(cause)]
        inner: serde_json::Error,
        path: PathBuf,
    },

    InvalidMetadataModelField {
        field_name: String,
        path: PathBuf,
    },

    MetadataClassNameNonInit {
        path: PathBuf,
    },

    XmlModelDecodeError {
        #[fail(cause)]
        inner: rbx_xml::DecodeError,
        path: PathBuf,
    },

    BinaryModelDecodeError {
        inner: rbx_binary::DecodeError,
        path: PathBuf,
    },

    CsvDecodeError {
        #[fail(cause)]
        inner: csv::Error,
        path: PathBuf,
    },

    ProjectNodeUnusable,

    ProjectNodeInvalidTransmute {
        partition_path: PathBuf,
    },

    PropertyResolveError {
        #[fail(cause)]
        inner: ValueResolveError,
    },
}

impl From<ValueResolveError> for SnapshotError {
    fn from(inner: ValueResolveError) -> SnapshotError {
        SnapshotError::PropertyResolveError {
            inner,
        }
    }
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, output: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SnapshotError::DidNotExist(path) => write!(output, "Path did not exist: {}", path.display()),
            SnapshotError::Utf8Error { inner, path } => {
                write!(output, "Invalid UTF-8: {} in path {}", inner, path.display())
            },
            SnapshotError::JsonModelDecodeError { inner, path } => {
                write!(output, "Malformed .model.json model: {} in path {}", inner, path.display())
            },
            SnapshotError::ExtraMetadataError { inner, path } => {
                write!(output, "Malformed init.meta.json: {} in path {}", inner, path.display())
            },
            SnapshotError::InvalidMetadataModelField { field_name, path } => {
                writeln!(output, "The field '{}' cannot be specified on .meta.json files attached to models.", field_name)?;
                writeln!(output, "Model path: {}", path.display())
            },
            SnapshotError::MetadataClassNameNonInit { path } => {
                writeln!(output, "The field 'className' cannot be specified on .meta.json files besides init.meta.json")?;
                writeln!(output, "Model path: {}", path.display())
            },
            SnapshotError::XmlModelDecodeError { inner, path } => {
                write!(output, "Malformed rbxmx model: {} in path {}", inner, path.display())
            },
            SnapshotError::BinaryModelDecodeError { inner, path } => {
                write!(output, "Malformed rbxm model: {:?} in path {}", inner, path.display())
            },
            SnapshotError::CsvDecodeError { inner, path } => {
                write!(output, "Malformed csv file: {} in path {}", inner, path.display())
            },
            SnapshotError::ProjectNodeUnusable => {
                write!(output, "Rojo project nodes must specify either $path or $className.")
            },
            SnapshotError::ProjectNodeInvalidTransmute { partition_path } => {
                writeln!(output, "Rojo project nodes that specify both $path and $className require that the")?;
                writeln!(output, "instance produced by the files pointed to by $path has a ClassName of")?;
                writeln!(output, "Folder.")?;
                writeln!(output, "")?;
                writeln!(output, "Partition target ($path): {}", partition_path.display())
            },
            SnapshotError::PropertyResolveError { inner } => write!(output, "{}", inner),
        }
    }
}

pub fn snapshot_project_tree<'source>(
    context: &SnapshotContext,
    imfs: &'source Imfs,
    project: &'source Project,
) -> SnapshotResult<'source> {
    snapshot_project_node(context, imfs, &project.tree, Cow::Borrowed(&project.name))
}

pub fn snapshot_project_node<'source>(
    context: &SnapshotContext,
    imfs: &'source Imfs,
    node: &ProjectNode,
    instance_name: Cow<'source, str>,
) -> SnapshotResult<'source> {
    let maybe_snapshot = match &node.path {
        Some(path) => snapshot_imfs_path(context, imfs, &path, Some(instance_name.clone()))?,
        None => match &node.class_name {
            Some(_class_name) => Some(RbxSnapshotInstance {
                name: instance_name.clone(),

                // These properties are replaced later in the function to
                // reduce code duplication.
                class_name: Cow::Borrowed("Folder"),
                properties: HashMap::new(),
                children: Vec::new(),
                metadata: MetadataPerInstance {
                    source_path: None,
                    ignore_unknown_instances: true,
                    project_definition: None,
                },
            }),
            None => {
                return Err(SnapshotError::ProjectNodeUnusable);
            },
        },
    };

    // If the snapshot resulted in no instances, like if it targets an unknown
    // file or an empty model file, we can early-return.
    //
    // In the future, we might want to issue a warning if the project also
    // specified fields like class_name, since the user will probably be
    // confused as to why nothing showed up in the tree.
    let mut snapshot = match maybe_snapshot {
        Some(snapshot) => snapshot,
        None => {
            // TODO: Return some other sort of marker here instead? If a node
            // transitions from None into Some, it's possible that configuration
            // from the ProjectNode might be lost since there's nowhere to put
            // it!
            return Ok(None);
        },
    };

    // Applies the class name specified in `class_name` from the project, if it's
    // set.
    if let Some(class_name) = &node.class_name {
        // This can only happen if `path` was specified in the project node and
        // that path represented a non-Folder instance.
        if snapshot.class_name != "Folder" {
            return Err(SnapshotError::ProjectNodeInvalidTransmute {
                partition_path: node.path.as_ref().unwrap().to_owned(),
            });
        }

        snapshot.class_name = Cow::Owned(class_name.to_owned());
    }

    for (child_name, child_project_node) in &node.children {
        if let Some(child) = snapshot_project_node(context, imfs, child_project_node, Cow::Owned(child_name.clone()))? {
            snapshot.children.push(child);
        }
    }

    for (key, value) in &node.properties {
        let resolved_value = try_resolve_value(&snapshot.class_name, key, value)?;
        snapshot.properties.insert(key.clone(), resolved_value);
    }

    if let Some(ignore_unknown_instances) = node.ignore_unknown_instances {
        snapshot.metadata.ignore_unknown_instances = ignore_unknown_instances;
    }

    snapshot.metadata.project_definition = Some((instance_name.into_owned(), node.clone()));

    Ok(Some(snapshot))
}

pub fn snapshot_imfs_path<'source>(
    context: &SnapshotContext,
    imfs: &'source Imfs,
    path: &Path,
    instance_name: Option<Cow<'source, str>>,
) -> SnapshotResult<'source> {
    // If the given path doesn't exist in the in-memory filesystem, we consider
    // that an error.
    match imfs.get(path) {
        Some(imfs_item) => snapshot_imfs_item(context, imfs, imfs_item, instance_name),
        None => return Err(SnapshotError::DidNotExist(path.to_owned())),
    }
}

fn snapshot_imfs_item<'source>(
    context: &SnapshotContext,
    imfs: &'source Imfs,
    item: &'source ImfsItem,
    instance_name: Option<Cow<'source, str>>,
) -> SnapshotResult<'source> {
    match item {
        ImfsItem::File(file) => snapshot_imfs_file(context, imfs, file, instance_name),
        ImfsItem::Directory(directory) => snapshot_imfs_directory(context, imfs, directory, instance_name),
    }
}

fn snapshot_imfs_directory<'source>(
    context: &SnapshotContext,
    imfs: &'source Imfs,
    directory: &'source ImfsDirectory,
    instance_name: Option<Cow<'source, str>>,
) -> SnapshotResult<'source> {
    let init_path = directory.path.join(INIT_MODULE_NAME);
    let init_server_path = directory.path.join(INIT_SERVER_NAME);
    let init_client_path = directory.path.join(INIT_CLIENT_NAME);

    let snapshot_name = instance_name
        .unwrap_or_else(|| {
            Cow::Borrowed(directory.path
                .file_name().expect("Could not extract file name")
                .to_str().expect("Could not convert path to UTF-8"))
        });

    let mut snapshot = if directory.children.contains(&init_path) {
        snapshot_imfs_path(context, imfs, &init_path, Some(snapshot_name))?.unwrap()
    } else if directory.children.contains(&init_server_path) {
        snapshot_imfs_path(context, imfs, &init_server_path, Some(snapshot_name))?.unwrap()
    } else if directory.children.contains(&init_client_path) {
        snapshot_imfs_path(context, imfs, &init_client_path, Some(snapshot_name))?.unwrap()
    } else {
        RbxSnapshotInstance {
            class_name: Cow::Borrowed("Folder"),
            name: snapshot_name,
            properties: HashMap::new(),
            children: Vec::new(),
            metadata: MetadataPerInstance {
                source_path: None,
                ignore_unknown_instances: false,
                project_definition: None,
            },
        }
    };

    if let Some(meta) = ExtraMetadata::locate(&imfs, &directory.path.join("init"))? {
        meta.apply(&mut snapshot)?;
    }

    snapshot.metadata.source_path = Some(directory.path.to_owned());

    for child_path in &directory.children {
        let child_name = child_path
            .file_name().expect("Couldn't extract file name")
            .to_str().expect("Couldn't convert file name to UTF-8");

        if child_name.ends_with(".meta.json") {
            // meta.json files don't turn into instances themselves, they just
            // modify other instances.
            continue;
        }

        match child_name {
            INIT_MODULE_NAME | INIT_SERVER_NAME | INIT_CLIENT_NAME => {
                // The existence of files with these names modifies the
                // parent instance and is handled above, so we can skip
                // them here.
                continue;
            }
            _ => {}
        }

        if let Some(child) = snapshot_imfs_path(context, imfs, child_path, None)? {
            snapshot.children.push(child);
        }
    }

    Ok(Some(snapshot))
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ExtraMetadata {
    class_name: Option<String>,
    ignore_unknown_instances: Option<bool>,

    #[serde(default = "HashMap::new")]
    properties: HashMap<String, UnresolvedRbxValue>,
}

impl ExtraMetadata {
    fn apply(self, snapshot: &mut RbxSnapshotInstance) -> Result<(), SnapshotError> {
        if let Some(meta_class) = self.class_name {
            snapshot.class_name = Cow::Owned(meta_class);
        }

        if let Some(meta_ignore_instances) = self.ignore_unknown_instances {
            snapshot.metadata.ignore_unknown_instances = meta_ignore_instances;
        }

        for (key, value) in self.properties {
            let resolved_value = try_resolve_value(&snapshot.class_name, &key, &value)?;
            snapshot.properties.insert(key, resolved_value);
        }

        Ok(())
    }

    fn locate(imfs: &Imfs, path: &Path) -> Result<Option<ExtraMetadata>, SnapshotError> {
        match imfs.get(&path.with_extension("meta.json")) {
            Some(ImfsItem::File(file)) => {
                let meta: ExtraMetadata = serde_json::from_slice(&file.contents)
                    .map_err(|inner| SnapshotError::ExtraMetadataError {
                        inner,
                        path: file.path.to_path_buf(),
                    })?;

                Ok(Some(meta))
            }
            _ => Ok(None)
        }
    }

    fn validate_for_non_init(&self, path: &Path) -> Result<(), SnapshotError> {
        if self.class_name.is_some() {
            return Err(SnapshotError::MetadataClassNameNonInit {
                path: path.to_owned(),
            });
        }

        Ok(())
    }

    fn validate_for_model(&self, path: &Path) -> Result<(), SnapshotError> {
        if self.class_name.is_some() {
            return Err(SnapshotError::InvalidMetadataModelField {
                field_name: "className".to_owned(),
                path: path.to_owned(),
            });
        }

        if !self.properties.is_empty() {
            return Err(SnapshotError::InvalidMetadataModelField {
                field_name: "properties".to_owned(),
                path: path.to_owned(),
            });
        }

        Ok(())
    }
}

fn snapshot_imfs_file<'source>(
    context: &SnapshotContext,
    imfs: &'source Imfs,
    file: &'source ImfsFile,
    instance_name: Option<Cow<'source, str>>,
) -> SnapshotResult<'source> {
    let extension = file.path.extension()
        .map(|v| v.to_str().expect("Could not convert extension to UTF-8"));

    let mut maybe_snapshot = match extension {
        Some("lua") => snapshot_lua_file(file, imfs)?,
        Some("csv") => snapshot_csv_file(file, imfs)?,
        Some("txt") => snapshot_txt_file(file, imfs)?,
        Some("rbxmx") => snapshot_xml_model_file(file, imfs)?,
        Some("rbxm") => snapshot_binary_model_file(file, imfs)?,
        Some("json") => {
            let file_stem = file.path
                .file_stem().expect("Could not extract file stem")
                .to_str().expect("Could not convert path to UTF-8");

            if file_stem.ends_with(".model") {
                snapshot_json_model_file(file)?
            } else {
                None
            }
        },
        Some(_) | None => None,
    };

    if let Some(mut snapshot) = maybe_snapshot.as_mut() {
        // Carefully preserve name from project manifest if present.
        if let Some(snapshot_name) = instance_name {
            snapshot.name = snapshot_name;
        }
    } else {
        info!("File generated no snapshot: {}", file.path.display());
    }

    if let Some(snapshot) = maybe_snapshot.as_ref() {
        if let Some(plugin_context) = &context.plugin_context {
            for plugin in &plugin_context.plugins {
                let owned_snapshot = snapshot.get_owned();
                let registry_key = &plugin.callback;

                plugin_context.lua.context(move |context| {
                    let callback: rlua::Function = context.registry_value(registry_key).unwrap();
                    callback.call::<_, ()>(LuaRbxSnapshot(owned_snapshot)).unwrap();
                });
            }
        }
    }

    Ok(maybe_snapshot)
}

fn snapshot_lua_file<'source>(
    file: &'source ImfsFile,
    imfs: &'source Imfs,
) -> SnapshotResult<'source> {
    let file_stem = file.path
        .file_stem().expect("Could not extract file stem")
        .to_str().expect("Could not convert path to UTF-8");

    let (instance_name, class_name) = if let Some(name) = match_trailing(file_stem, ".server") {
        (name, "Script")
    } else if let Some(name) = match_trailing(file_stem, ".client") {
        (name, "LocalScript")
    } else {
        (file_stem, "ModuleScript")
    };

    let contents = str::from_utf8(&file.contents)
        .map_err(|inner| SnapshotError::Utf8Error {
            inner,
            path: file.path.to_path_buf(),
        })?;

    let mut snapshot = RbxSnapshotInstance {
        name: Cow::Borrowed(instance_name),
        class_name: Cow::Borrowed(class_name),
        properties: hashmap! {
            "Source".to_owned() => RbxValue::String {
                value: contents.to_owned(),
            },
        },
        children: Vec::new(),
        metadata: MetadataPerInstance {
            source_path: Some(file.path.to_path_buf()),
            ignore_unknown_instances: false,
            project_definition: None,
        },
    };

    if let Some(meta) = ExtraMetadata::locate(&imfs, &file.path.with_file_name(instance_name))? {
        meta.validate_for_non_init(&file.path)?;
        meta.apply(&mut snapshot)?;
    }

    Ok(Some(snapshot))
}

fn match_trailing<'a>(input: &'a str, trailer: &str) -> Option<&'a str> {
    if input.ends_with(trailer) {
        let end = input.len().saturating_sub(trailer.len());
        Some(&input[..end])
    } else {
        None
    }
}

fn snapshot_txt_file<'source>(
    file: &'source ImfsFile,
    imfs: &'source Imfs,
) -> SnapshotResult<'source> {
    let instance_name = file.path
        .file_stem().expect("Could not extract file stem")
        .to_str().expect("Could not convert path to UTF-8");

    let contents = str::from_utf8(&file.contents)
        .map_err(|inner| SnapshotError::Utf8Error {
            inner,
            path: file.path.to_path_buf(),
        })?;

    let mut snapshot = RbxSnapshotInstance {
        name: Cow::Borrowed(instance_name),
        class_name: Cow::Borrowed("StringValue"),
        properties: hashmap! {
            "Value".to_owned() => RbxValue::String {
                value: contents.to_owned(),
            },
        },
        children: Vec::new(),
        metadata: MetadataPerInstance {
            source_path: Some(file.path.to_path_buf()),
            ignore_unknown_instances: false,
            project_definition: None,
        },
    };

    if let Some(meta) = ExtraMetadata::locate(&imfs, &file.path)? {
        meta.validate_for_non_init(&file.path)?;
        meta.apply(&mut snapshot)?;
    }

    Ok(Some(snapshot))
}

fn snapshot_csv_file<'source>(
    file: &'source ImfsFile,
    imfs: &'source Imfs,
) -> SnapshotResult<'source> {
    /// Struct that holds any valid row from a Roblox CSV translation table.
    ///
    /// We manually deserialize into this table from CSV, but let JSON handle
    /// serializing.
    #[derive(Debug, Default, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct LocalizationEntry<'a> {
        #[serde(skip_serializing_if = "Option::is_none")]
        key: Option<&'a str>,

        #[serde(skip_serializing_if = "Option::is_none")]
        context: Option<&'a str>,

        #[serde(skip_serializing_if = "Option::is_none")]
        example: Option<&'a str>,

        #[serde(skip_serializing_if = "Option::is_none")]
        source: Option<&'a str>,

        // We use a BTreeMap here to get deterministic output order.
        values: BTreeMap<&'a str, &'a str>,
    }

    let instance_name = file.path
        .file_stem().expect("Could not extract file stem")
        .to_str().expect("Could not convert path to UTF-8");

    // Normally, we'd be able to let the csv crate construct our struct for us.
    //
    // However, because of a limitation with Serde's 'flatten' feature, it's not
    // possible presently to losslessly collect extra string values while using
    // csv+Serde.
    //
    // https://github.com/BurntSushi/rust-csv/issues/151
    let mut reader = csv::Reader::from_reader(file.contents.as_slice());

    let headers = reader.headers()
        .map_err(|inner| SnapshotError::CsvDecodeError {
            inner,
            path: file.path.to_path_buf(),
        })?
        .clone();

    let mut records = Vec::new();

    for record in reader.into_records() {
        let record = record
            .map_err(|inner| SnapshotError::CsvDecodeError {
                inner,
                path: file.path.to_path_buf(),
            })?;

        records.push(record);
    }

    let mut entries = Vec::new();

    for record in &records {
        let mut entry = LocalizationEntry::default();

        for (header, value) in headers.iter().zip(record.into_iter()) {
            if header.is_empty() || value.is_empty() {
                continue;
            }

            match header {
                "Key" => entry.key = Some(value),
                "Source" => entry.source = Some(value),
                "Context" => entry.context = Some(value),
                "Example" => entry.example = Some(value),
                _ => {
                    entry.values.insert(header, value);
                }
            }
        }

        if entry.key.is_none() && entry.source.is_none() {
            continue;
        }

        entries.push(entry);
    }

    let table_contents = serde_json::to_string(&entries)
        .expect("Could not encode JSON for localization table");

    let mut snapshot = RbxSnapshotInstance {
        name: Cow::Borrowed(instance_name),
        class_name: Cow::Borrowed("LocalizationTable"),
        properties: hashmap! {
            "Contents".to_owned() => RbxValue::String {
                value: table_contents,
            },
        },
        children: Vec::new(),
        metadata: MetadataPerInstance {
            source_path: Some(file.path.to_path_buf()),
            ignore_unknown_instances: false,
            project_definition: None,
        },
    };

    if let Some(meta) = ExtraMetadata::locate(&imfs, &file.path)? {
        meta.validate_for_non_init(&file.path)?;
        meta.apply(&mut snapshot)?;
    }

    Ok(Some(snapshot))
}

fn snapshot_json_model_file<'source>(
    file: &'source ImfsFile,
) -> SnapshotResult<'source> {
    let file_name = file.path
        .file_name().expect("Could not extract file stem")
        .to_str().expect("Could not convert path to UTF-8");

    let instance_name = match_trailing(file_name, ".model.json")
        .expect("JSON model file did not end in .model.json");

    let contents = str::from_utf8(&file.contents)
        .map_err(|inner| SnapshotError::Utf8Error {
            inner,
            path: file.path.to_owned(),
        })?;

    let json_instance: JsonModel = serde_json::from_str(contents)
        .map_err(|inner| SnapshotError::JsonModelDecodeError {
            inner,
            path: file.path.to_owned(),
        })?;

    if let Some(json_name) = &json_instance.name {
        if json_name != instance_name {
            log::warn!("Name from JSON model did not match its file name: {}", file.path.display());
            log::warn!("In Rojo <  alpha 14, this model is named \"{}\" (from its 'Name' property)", json_name);
            log::warn!("In Rojo >= alpha 14, this model is named \"{}\" (from its file name)", instance_name);
            log::warn!("'Name' for the top-level instance in a JSON model is now optional and will be ignored.");
        }
    }

    let mut snapshot = json_instance.core.into_snapshot(instance_name.to_owned())?;

    snapshot.name = Cow::Borrowed(instance_name);
    snapshot.metadata.source_path = Some(file.path.to_owned());

    Ok(Some(snapshot))
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct JsonModel {
    name: Option<String>,

    #[serde(flatten)]
    core: JsonModelCore,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct JsonModelInstance {
    name: String,

    #[serde(flatten)]
    core: JsonModelCore,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct JsonModelCore {
    class_name: String,

    #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
    children: Vec<JsonModelInstance>,

    #[serde(default = "HashMap::new", skip_serializing_if = "HashMap::is_empty")]
    properties: HashMap<String, UnresolvedRbxValue>,
}

impl JsonModelCore {
    fn into_snapshot(self, name: String) -> Result<RbxSnapshotInstance<'static>, SnapshotError> {
        let mut children = Vec::with_capacity(self.children.len());

        for child in self.children {
            children.push(child.core.into_snapshot(child.name)?);
        }

        let mut properties = HashMap::with_capacity(self.properties.len());

        for (key, value) in self.properties {
            let resolved_value = try_resolve_value(&self.class_name, &key, &value)?;
            properties.insert(key, resolved_value);
        }

        Ok(RbxSnapshotInstance {
            name: Cow::Owned(name),
            class_name: Cow::Owned(self.class_name),
            properties,
            children,
            metadata: Default::default(),
        })
    }
}

fn snapshot_xml_model_file<'source>(
    file: &'source ImfsFile,
    imfs: &'source Imfs,
) -> SnapshotResult<'source> {
    let instance_name = file.path
        .file_stem().expect("Could not extract file stem")
        .to_str().expect("Could not convert path to UTF-8");

    let options = rbx_xml::DecodeOptions::new()
        .property_behavior(rbx_xml::DecodePropertyBehavior::ReadUnknown);

    let temp_tree = rbx_xml::from_reader(file.contents.as_slice(), options)
        .map_err(|inner| SnapshotError::XmlModelDecodeError {
            inner,
            path: file.path.clone(),
        })?;

    let root_instance = temp_tree.get_instance(temp_tree.get_root_id()).unwrap();
    let children = root_instance.get_children_ids();

    match children.len() {
        0 => Ok(None),
        1 => {
            let mut snapshot = snapshot_from_tree(&temp_tree, children[0]).unwrap();
            snapshot.name = Cow::Borrowed(instance_name);
            snapshot.metadata.source_path = Some(file.path.clone());

            if let Some(meta) = ExtraMetadata::locate(&imfs, &file.path)? {
                meta.validate_for_model(&file.path)?;
                meta.apply(&mut snapshot)?;
            }

            Ok(Some(snapshot))
        },
        _ => panic!("Rojo doesn't have support for model files with multiple roots yet"),
    }
}

fn snapshot_binary_model_file<'source>(
    file: &'source ImfsFile,
    imfs: &'source Imfs,
) -> SnapshotResult<'source> {
    let instance_name = file.path
        .file_stem().expect("Could not extract file stem")
        .to_str().expect("Could not convert path to UTF-8");

    let mut temp_tree = RbxTree::new(RbxInstanceProperties {
        name: "Temp".to_owned(),
        class_name: "Folder".to_owned(),
        properties: HashMap::new(),
    });

    let root_id = temp_tree.get_root_id();
    rbx_binary::decode(&mut temp_tree, root_id, file.contents.as_slice())
        .map_err(|inner| SnapshotError::BinaryModelDecodeError {
            inner,
            path: file.path.clone(),
        })?;

    let root_instance = temp_tree.get_instance(root_id).unwrap();
    let children = root_instance.get_children_ids();

    match children.len() {
        0 => Ok(None),
        1 => {
            let mut snapshot = snapshot_from_tree(&temp_tree, children[0]).unwrap();
            snapshot.name = Cow::Borrowed(instance_name);
            snapshot.metadata.source_path = Some(file.path.clone());

            if let Some(meta) = ExtraMetadata::locate(&imfs, &file.path)? {
                meta.validate_for_model(&file.path)?;
                meta.apply(&mut snapshot)?;
            }

            Ok(Some(snapshot))
        },
        _ => panic!("Rojo doesn't have support for model files with multiple roots yet"),
    }
}