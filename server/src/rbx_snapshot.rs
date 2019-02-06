use std::{
    borrow::Cow,
    collections::HashMap,
    fmt,
    path::{Path, PathBuf},
    str,
};

use failure::Fail;
use log::info;
use maplit::hashmap;
use rbx_tree::{RbxTree, RbxValue, RbxInstanceProperties};
use serde_derive::{Serialize, Deserialize};

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

pub type SnapshotResult<'a> = Result<Option<RbxSnapshotInstance<'a>>, SnapshotError>;

pub struct SnapshotContext {
}

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

    XmlModelDecodeError {
        inner: rbx_xml::DecodeError,
        path: PathBuf,
    },

    BinaryModelDecodeError {
        inner: rbx_binary::DecodeError,
        path: PathBuf,
    },

    ProjectNodeUnusable,

    ProjectNodeInvalidTransmute {
        partition_path: PathBuf,
    },
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
            SnapshotError::XmlModelDecodeError { inner, path } => {
                write!(output, "Malformed rbxmx model: {:?} in path {}", inner, path.display())
            },
            SnapshotError::BinaryModelDecodeError { inner, path } => {
                write!(output, "Malformed rbxm model: {:?} in path {}", inner, path.display())
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
        }
    }
}

pub fn snapshot_project_tree<'source>(
    imfs: &'source Imfs,
    context: &SnapshotContext,
    project: &'source Project,
) -> SnapshotResult<'source> {
    snapshot_project_node(imfs, context, &project.tree, Cow::Borrowed(&project.name))
}

fn snapshot_project_node<'source>(
    imfs: &'source Imfs,
    context: &SnapshotContext,
    node: &'source ProjectNode,
    instance_name: Cow<'source, str>,
) -> SnapshotResult<'source> {
    let maybe_snapshot = match &node.path {
        Some(path) => snapshot_imfs_path(imfs, context, &path, Some(instance_name))?,
        None => match &node.class_name {
            Some(_class_name) => {
                let name_from_above = instance_name.clone().into_owned();

                Some(RbxSnapshotInstance {
                    name: instance_name,

                    // These properties are replaced later in the function to
                    // reduce code duplication.
                    class_name: Cow::Borrowed("Folder"),
                    properties: HashMap::new(),
                    children: Vec::new(),
                    metadata: MetadataPerInstance {
                        source_path: None,
                        ignore_unknown_instances: true,
                        instance_name: Some(name_from_above),
                    },
                })
            },
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
        None => return Ok(None),
    };

    for (child_name, child_project_node) in &node.children {
        if let Some(child) = snapshot_project_node(imfs, context, child_project_node, Cow::Borrowed(child_name))? {
            snapshot.children.push(child);
        }
    }

    if let Some(class_name) = &node.class_name {
        if snapshot.class_name != "Folder" {
            return Err(SnapshotError::ProjectNodeInvalidTransmute {
                partition_path: node.path.as_ref().unwrap().to_owned(),
            });
        }

        snapshot.class_name = Cow::Borrowed(&class_name);
    }

    for (key, value) in &node.properties {
        snapshot.properties.insert(key.clone(), value.clone());
    }

    if let Some(ignore_unknown_instances) = node.ignore_unknown_instances {
        snapshot.metadata.ignore_unknown_instances = ignore_unknown_instances;
    }

    Ok(Some(snapshot))
}

pub fn snapshot_imfs_path<'source>(
    imfs: &'source Imfs,
    context: &SnapshotContext,
    path: &Path,
    instance_name: Option<Cow<'source, str>>,
) -> SnapshotResult<'source> {
    // If the given path doesn't exist in the in-memory filesystem, we consider
    // that an error.
    match imfs.get(path) {
        Some(imfs_item) => snapshot_imfs_item(imfs, context, imfs_item, instance_name),
        None => return Err(SnapshotError::DidNotExist(path.to_owned())),
    }
}

fn snapshot_imfs_item<'source>(
    imfs: &'source Imfs,
    context: &SnapshotContext,
    item: &'source ImfsItem,
    instance_name: Option<Cow<'source, str>>,
) -> SnapshotResult<'source> {
    match item {
        ImfsItem::File(file) => snapshot_imfs_file(file, instance_name),
        ImfsItem::Directory(directory) => snapshot_imfs_directory(imfs, context, directory, instance_name),
    }
}

fn snapshot_imfs_directory<'source>(
    imfs: &'source Imfs,
    context: &SnapshotContext,
    directory: &'source ImfsDirectory,
    instance_name: Option<Cow<'source, str>>,
) -> SnapshotResult<'source> {
    let init_path = directory.path.join(INIT_MODULE_NAME);
    let init_server_path = directory.path.join(INIT_SERVER_NAME);
    let init_client_path = directory.path.join(INIT_CLIENT_NAME);

    let name_from_above = instance_name.as_ref()
        .map(|inner| inner.clone().into_owned());

    let snapshot_name = instance_name
        .unwrap_or_else(|| {
            Cow::Borrowed(directory.path
                .file_name().expect("Could not extract file name")
                .to_str().expect("Could not convert path to UTF-8"))
        });

    let mut snapshot = if directory.children.contains(&init_path) {
        snapshot_imfs_path(imfs, context, &init_path, Some(snapshot_name))?.unwrap()
    } else if directory.children.contains(&init_server_path) {
        snapshot_imfs_path(imfs, context, &init_server_path, Some(snapshot_name))?.unwrap()
    } else if directory.children.contains(&init_client_path) {
        snapshot_imfs_path(imfs, context, &init_client_path, Some(snapshot_name))?.unwrap()
    } else {
        RbxSnapshotInstance {
            class_name: Cow::Borrowed("Folder"),
            name: snapshot_name,
            properties: HashMap::new(),
            children: Vec::new(),
            metadata: MetadataPerInstance {
                source_path: None,
                ignore_unknown_instances: false,
                instance_name: name_from_above,
            },
        }
    };

    snapshot.metadata.source_path = Some(directory.path.to_owned());

    for child_path in &directory.children {
        let child_name = child_path
            .file_name().expect("Couldn't extract file name")
            .to_str().expect("Couldn't convert file name to UTF-8");

        match child_name {
            INIT_MODULE_NAME | INIT_SERVER_NAME | INIT_CLIENT_NAME => {
                // The existence of files with these names modifies the
                // parent instance and is handled above, so we can skip
                // them here.
            },
            _ => {
                if let Some(child) = snapshot_imfs_path(imfs, context, child_path, None)? {
                    snapshot.children.push(child);
                }
            },
        }
    }

    Ok(Some(snapshot))
}

fn snapshot_imfs_file<'source>(
    file: &'source ImfsFile,
    instance_name: Option<Cow<'source, str>>,
) -> SnapshotResult<'source> {
    let extension = file.path.extension()
        .map(|v| v.to_str().expect("Could not convert extension to UTF-8"));

    let mut maybe_snapshot = match extension {
        Some("lua") => snapshot_lua_file(file)?,
        Some("csv") => snapshot_csv_file(file)?,
        Some("txt") => snapshot_txt_file(file)?,
        Some("rbxmx") => snapshot_xml_model_file(file)?,
        Some("rbxm") => snapshot_binary_model_file(file)?,
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

    if let Some(snapshot) = maybe_snapshot.as_mut() {
        // Carefully preserve name from project manifest if present.
        if let Some(snapshot_name) = instance_name {
            snapshot.name = snapshot_name;
        }
    } else {
        info!("File generated no snapshot: {}", file.path.display());
    }

    Ok(maybe_snapshot)
}

fn snapshot_lua_file<'source>(
    file: &'source ImfsFile,
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

    Ok(Some(RbxSnapshotInstance {
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
            instance_name: None,
        },
    }))
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
) -> SnapshotResult<'source> {
    let instance_name = file.path
        .file_stem().expect("Could not extract file stem")
        .to_str().expect("Could not convert path to UTF-8");

    let contents = str::from_utf8(&file.contents)
        .map_err(|inner| SnapshotError::Utf8Error {
            inner,
            path: file.path.to_path_buf(),
        })?;

    Ok(Some(RbxSnapshotInstance {
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
            instance_name: None,
        },
    }))
}

fn snapshot_csv_file<'source>(
    file: &'source ImfsFile,
) -> SnapshotResult<'source> {
    let instance_name = file.path
        .file_stem().expect("Could not extract file stem")
        .to_str().expect("Could not convert path to UTF-8");

    let entries: Vec<LocalizationEntryJson> = csv::Reader::from_reader(file.contents.as_slice())
        .deserialize()
        // TODO: Propagate error upward instead of panicking
        .map(|result| result.expect("Malformed localization table found!"))
        .map(LocalizationEntryCsv::to_json)
        .collect();

    let table_contents = serde_json::to_string(&entries)
        .expect("Could not encode JSON for localization table");

    Ok(Some(RbxSnapshotInstance {
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
            instance_name: None,
        },
    }))
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct LocalizationEntryCsv {
    key: String,
    context: String,
    example: String,
    source: String,
    #[serde(flatten)]
    values: HashMap<String, String>,
}

impl LocalizationEntryCsv {
    fn to_json(self) -> LocalizationEntryJson {
        LocalizationEntryJson {
            key: self.key,
            context: self.context,
            example: self.example,
            source: self.source,
            values: self.values,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocalizationEntryJson {
    key: String,
    context: String,
    example: String,
    source: String,
    values: HashMap<String, String>,
}

fn snapshot_json_model_file<'source>(
    file: &'source ImfsFile,
) -> SnapshotResult<'source> {
    let contents = str::from_utf8(&file.contents)
        .map_err(|inner| SnapshotError::Utf8Error {
            inner,
            path: file.path.to_owned(),
        })?;

    let json_instance: JsonModelInstance = serde_json::from_str(contents)
        .map_err(|inner| SnapshotError::JsonModelDecodeError {
            inner,
            path: file.path.to_owned(),
        })?;

    let mut snapshot = json_instance.into_snapshot();
    snapshot.metadata.source_path = Some(file.path.to_owned());

    Ok(Some(snapshot))
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct JsonModelInstance {
    name: String,
    class_name: String,

    #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
    children: Vec<JsonModelInstance>,

    #[serde(default = "HashMap::new", skip_serializing_if = "HashMap::is_empty")]
    properties: HashMap<String, RbxValue>,
}

impl JsonModelInstance {
    fn into_snapshot(mut self) -> RbxSnapshotInstance<'static> {
        let children = self.children
            .drain(..)
            .map(JsonModelInstance::into_snapshot)
            .collect();

        RbxSnapshotInstance {
            name: Cow::Owned(self.name),
            class_name: Cow::Owned(self.class_name),
            properties: self.properties,
            children,
            metadata: Default::default(),
        }
    }
}

fn snapshot_xml_model_file<'source>(
    file: &'source ImfsFile,
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
    rbx_xml::decode(&mut temp_tree, root_id, file.contents.as_slice())
        .map_err(|inner| SnapshotError::XmlModelDecodeError {
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
            Ok(Some(snapshot))
        },
        _ => panic!("Rojo doesn't have support for model files with multiple roots yet"),
    }
}

fn snapshot_binary_model_file<'source>(
    file: &'source ImfsFile,
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
            Ok(Some(snapshot))
        },
        _ => panic!("Rojo doesn't have support for model files with multiple roots yet"),
    }
}