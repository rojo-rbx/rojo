use std::{
    borrow::Cow,
    collections::HashMap,
    fmt,
    path::{Path, PathBuf},
    str,
    sync::{Arc, Mutex},
};

use failure::Fail;

use rbx_tree::{RbxTree, RbxValue, RbxId};

use crate::{
    project::{Project, ProjectNode, InstanceProjectNodeMetadata},
    message_queue::MessageQueue,
    imfs::{Imfs, ImfsItem, ImfsFile},
    path_map::PathMap,
    rbx_snapshot::{RbxSnapshotInstance, InstanceChanges, reify_root, reconcile_subtree},
};

const INIT_SCRIPT: &str = "init.lua";
const INIT_SERVER_SCRIPT: &str = "init.server.lua";
const INIT_CLIENT_SCRIPT: &str = "init.client.lua";

pub struct RbxSession {
    tree: RbxTree,
    path_map: PathMap<RbxId>,
    instance_metadata_map: HashMap<RbxId, InstanceProjectNodeMetadata>,
    sync_point_names: HashMap<PathBuf, String>,
    message_queue: Arc<MessageQueue<InstanceChanges>>,
    imfs: Arc<Mutex<Imfs>>,
}

impl RbxSession {
    pub fn new(
        project: Arc<Project>,
        imfs: Arc<Mutex<Imfs>>,
        message_queue: Arc<MessageQueue<InstanceChanges>>,
    ) -> RbxSession {
        let mut sync_point_names = HashMap::new();
        let mut path_map = PathMap::new();
        let mut instance_metadata_map = HashMap::new();

        let tree = {
            let temp_imfs = imfs.lock().unwrap();
            construct_initial_tree(&project, &temp_imfs, &mut path_map, &mut instance_metadata_map, &mut sync_point_names)
        };

        RbxSession {
            tree,
            path_map,
            instance_metadata_map,
            sync_point_names,
            message_queue,
            imfs,
        }
    }

    fn path_created_or_updated(&mut self, path: &Path) {
        // TODO: Track paths actually updated in each step so we can ignore
        // redundant changes.
        let mut changes = InstanceChanges::default();

        {
            let imfs = self.imfs.lock().unwrap();
            let root_path = imfs.get_root_for_path(path)
                .expect("Path was outside in-memory filesystem roots");

            // Find the closest instance in the tree that currently exists
            let mut path_to_snapshot = self.path_map.descend(root_path, path);
            let &instance_id = self.path_map.get(&path_to_snapshot).unwrap();

            // If this is a file that might affect its parent if modified, we
            // should snapshot its parent instead.
            match path_to_snapshot.file_name().unwrap().to_str() {
                Some(INIT_SCRIPT) | Some(INIT_SERVER_SCRIPT) | Some(INIT_CLIENT_SCRIPT) => {
                    path_to_snapshot.pop();
                },
                _ => {},
            }

            trace!("Snapshotting path {}", path_to_snapshot.display());

            let maybe_snapshot = snapshot_instances_from_imfs(&imfs, &path_to_snapshot, &mut self.sync_point_names)
                .unwrap_or_else(|_| panic!("Could not generate instance snapshot for path {}", path_to_snapshot.display()));

            let snapshot = match maybe_snapshot {
                Some(snapshot) => snapshot,
                None => {
                    trace!("Path resulted in no snapshot being generated.");
                    return;
                },
            };

            trace!("Snapshot: {:#?}", snapshot);

            reconcile_subtree(
                &mut self.tree,
                instance_id,
                &snapshot,
                &mut self.path_map,
                &mut self.instance_metadata_map,
                &mut changes,
            );
        }

        if changes.is_empty() {
            trace!("No instance changes triggered from file update.");
        } else {
            trace!("Pushing changes: {}", changes);
            self.message_queue.push_messages(&[changes]);
        }
    }

    pub fn path_created(&mut self, path: &Path) {
        info!("Path created: {}", path.display());
        self.path_created_or_updated(path);
    }

    pub fn path_updated(&mut self, path: &Path) {
        info!("Path updated: {}", path.display());

        {
            let imfs = self.imfs.lock().unwrap();

            // If the path doesn't exist or is a directory, we don't care if it
            // updated
            match imfs.get(path) {
                Some(ImfsItem::Directory(_)) | None => {
                    trace!("Updated path was a directory, ignoring.");
                    return;
                },
                Some(ImfsItem::File(_)) => {},
            }
        }

        self.path_created_or_updated(path);
    }

    pub fn path_removed(&mut self, path: &Path) {
        info!("Path removed: {}", path.display());
        self.path_map.remove(path);
        self.path_created_or_updated(path);
    }

    pub fn path_renamed(&mut self, from_path: &Path, to_path: &Path) {
        info!("Path renamed from {} to {}", from_path.display(), to_path.display());
        self.path_map.remove(from_path);
        self.path_created_or_updated(from_path);
        self.path_created_or_updated(to_path);
    }

    pub fn get_tree(&self) -> &RbxTree {
        &self.tree
    }

    pub fn get_instance_metadata_map(&self) -> &HashMap<RbxId, InstanceProjectNodeMetadata> {
        &self.instance_metadata_map
    }

    pub fn debug_get_path_map(&self) -> &PathMap<RbxId> {
        &self.path_map
    }
}

pub fn construct_oneoff_tree(project: &Project, imfs: &Imfs) -> RbxTree {
    let mut path_map = PathMap::new();
    let mut instance_metadata_map = HashMap::new();
    let mut sync_point_names = HashMap::new();
    construct_initial_tree(project, imfs, &mut path_map, &mut instance_metadata_map, &mut sync_point_names)
}

fn construct_initial_tree(
    project: &Project,
    imfs: &Imfs,
    path_map: &mut PathMap<RbxId>,
    instance_metadata_map: &mut HashMap<RbxId, InstanceProjectNodeMetadata>,
    sync_point_names: &mut HashMap<PathBuf, String>,
) -> RbxTree {
    let snapshot = construct_project_node(
        imfs,
        &project.name,
        &project.tree,
        sync_point_names,
    );

    let mut changes = InstanceChanges::default();
    let tree = reify_root(&snapshot, path_map, instance_metadata_map, &mut changes);

    tree
}

fn construct_project_node<'a>(
    imfs: &'a Imfs,
    instance_name: &'a str,
    project_node: &'a ProjectNode,
    sync_point_names: &mut HashMap<PathBuf, String>,
) -> RbxSnapshotInstance<'a> {
    match project_node {
        ProjectNode::Instance(node) => {
            let mut children = Vec::new();

            for (child_name, child_project_node) in &node.children {
                children.push(construct_project_node(imfs, child_name, child_project_node, sync_point_names));
            }

            RbxSnapshotInstance {
                class_name: Cow::Borrowed(&node.class_name),
                name: Cow::Borrowed(instance_name),
                properties: node.properties.clone(),
                children,
                source_path: None,
                metadata: Some(node.metadata.clone()),
            }
        },
        ProjectNode::SyncPoint(node) => {
            // TODO: Propagate errors upward instead of dying
            let mut snapshot = snapshot_instances_from_imfs(imfs, &node.path, sync_point_names)
                .expect("Could not reify nodes from Imfs")
                .expect("Sync point node did not result in an instance");

            snapshot.name = Cow::Borrowed(instance_name);
            sync_point_names.insert(node.path.clone(), instance_name.to_string());

            snapshot
        },
    }
}

#[derive(Debug, Clone, Copy)]
enum FileType {
    ModuleScript,
    ServerScript,
    ClientScript,
    StringValue,
    LocalizationTable,
}

fn get_trailing<'a>(input: &'a str, trailer: &str) -> Option<&'a str> {
    if input.ends_with(trailer) {
        let end = input.len().saturating_sub(trailer.len());
        Some(&input[..end])
    } else {
        None
    }
}

fn classify_file(file: &ImfsFile) -> Option<(&str, FileType)> {
    let file_name = file.path.file_name()?.to_str()?;

    if let Some(instance_name) = get_trailing(file_name, ".server.lua") {
        Some((instance_name, FileType::ServerScript))
    } else if let Some(instance_name) = get_trailing(file_name, ".client.lua") {
        Some((instance_name, FileType::ClientScript))
    } else if let Some(instance_name) = get_trailing(file_name, ".lua") {
        Some((instance_name, FileType::ModuleScript))
    } else if let Some(instance_name) = get_trailing(file_name, ".csv") {
        Some((instance_name, FileType::LocalizationTable))
    } else if let Some(instance_name) = get_trailing(file_name, ".txt") {
        Some((instance_name, FileType::StringValue))
    } else {
        None
    }
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

#[derive(Debug, Fail)]
enum SnapshotError {
    DidNotExist(PathBuf),

    // TODO: Add file path to the error message?
    Utf8Error {
        #[fail(cause)]
        inner: str::Utf8Error,
        path: PathBuf,
    },
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, output: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SnapshotError::DidNotExist(path) => write!(output, "Path did not exist: {}", path.display()),
            SnapshotError::Utf8Error { inner, path } => {
                write!(output, "Invalid UTF-8: {} in path {}", inner, path.display())
            },
        }
    }
}

fn snapshot_instances_from_imfs<'a>(
    imfs: &'a Imfs,
    imfs_path: &Path,
    sync_point_names: &HashMap<PathBuf, String>,
) -> Result<Option<RbxSnapshotInstance<'a>>, SnapshotError> {
    match imfs.get(imfs_path) {
        Some(ImfsItem::File(file)) => {
            let (instance_name, file_type) = match classify_file(file) {
                Some(info) => info,
                None => return Ok(None),
            };

            let instance_name = if let Some(actual_name) = sync_point_names.get(imfs_path) {
                Cow::Owned(actual_name.clone())
            } else {
                Cow::Borrowed(instance_name)
            };

            let class_name = match file_type {
                FileType::ModuleScript => "ModuleScript",
                FileType::ServerScript => "Script",
                FileType::ClientScript => "LocalScript",
                FileType::StringValue => "StringValue",
                FileType::LocalizationTable => "LocalizationTable",
            };

            let contents = str::from_utf8(&file.contents)
                .map_err(|inner| SnapshotError::Utf8Error {
                    inner,
                    path: imfs_path.to_path_buf(),
                })?;

            let mut properties = HashMap::new();

            match file_type {
                FileType::ModuleScript | FileType::ServerScript | FileType::ClientScript => {
                    properties.insert(String::from("Source"), RbxValue::String {
                        value: contents.to_string(),
                    });
                },
                FileType::StringValue => {
                    properties.insert(String::from("Value"), RbxValue::String {
                        value: contents.to_string(),
                    });
                },
                FileType::LocalizationTable => {
                    let entries: Vec<LocalizationEntryJson> = csv::Reader::from_reader(contents.as_bytes())
                        .deserialize()
                        .map(|result| result.expect("Malformed localization table found!"))
                        .map(LocalizationEntryCsv::to_json)
                        .collect();

                    let table_contents = serde_json::to_string(&entries)
                        .expect("Could not encode JSON for localization table");

                    properties.insert(String::from("Contents"), RbxValue::String {
                        value: table_contents,
                    });
                },
            }

            Ok(Some(RbxSnapshotInstance {
                name: instance_name,
                class_name: Cow::Borrowed(class_name),
                properties,
                children: Vec::new(),
                source_path: Some(file.path.clone()),
                metadata: None,
            }))
        },
        Some(ImfsItem::Directory(directory)) => {
            // TODO: Expand init support to handle server and client scripts
            let init_path = directory.path.join(INIT_SCRIPT);
            let init_server_path = directory.path.join(INIT_SERVER_SCRIPT);
            let init_client_path = directory.path.join(INIT_CLIENT_SCRIPT);

            let mut instance = if directory.children.contains(&init_path) {
                snapshot_instances_from_imfs(imfs, &init_path, sync_point_names)?
                    .expect("Could not snapshot instance from file that existed!")
            } else if directory.children.contains(&init_server_path) {
                snapshot_instances_from_imfs(imfs, &init_server_path, sync_point_names)?
                    .expect("Could not snapshot instance from file that existed!")
            } else if directory.children.contains(&init_client_path) {
                snapshot_instances_from_imfs(imfs, &init_client_path, sync_point_names)?
                    .expect("Could not snapshot instance from file that existed!")
            } else {
                RbxSnapshotInstance {
                    class_name: Cow::Borrowed("Folder"),
                    name: Cow::Borrowed(""),
                    properties: HashMap::new(),
                    children: Vec::new(),
                    source_path: Some(directory.path.clone()),
                    metadata: None,
                }
            };

            // We have to be careful not to lose instance names that are
            // specified in the project manifest. We store them in
            // sync_point_names when the original tree is constructed.
            instance.name = if let Some(actual_name) = sync_point_names.get(&directory.path) {
                Cow::Owned(actual_name.clone())
            } else {
                Cow::Borrowed(directory.path
                    .file_name().expect("Could not extract file name")
                    .to_str().expect("Could not convert path to UTF-8"))
            };

            for child_path in &directory.children {
                match child_path.file_name().unwrap().to_str().unwrap() {
                    INIT_SCRIPT | INIT_SERVER_SCRIPT | INIT_CLIENT_SCRIPT => {
                        // The existence of files with these names modifies the
                        // parent instance and is handled above, so we can skip
                        // them here.
                    },
                    _ => {
                        match snapshot_instances_from_imfs(imfs, child_path, sync_point_names)? {
                            Some(child) => {
                                instance.children.push(child);
                            },
                            None => {},
                        }
                    },
                }
            }

            Ok(Some(instance))
        },
        None => Err(SnapshotError::DidNotExist(imfs_path.to_path_buf())),
    }
}