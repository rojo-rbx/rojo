use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    str,
};

use rbx_tree::{RbxTree, RbxValue, RbxId};

use crate::{
    project::{Project, ProjectNode, InstanceProjectNode, InstanceProjectNodeMetadata},
    message_queue::MessageQueue,
    imfs::{Imfs, ImfsItem, ImfsFile},
    path_map::PathMap,
    rbx_snapshot::{RbxSnapshotInstance, InstanceChanges, reify_root, reconcile_subtree},
};

pub struct RbxSession {
    tree: RbxTree,
    path_map: PathMap<RbxId>,
    instance_metadata_map: HashMap<RbxId, InstanceProjectNodeMetadata>,
    sync_point_names: HashMap<PathBuf, String>,
    message_queue: Arc<MessageQueue<InstanceChanges>>,
    imfs: Arc<Mutex<Imfs>>,
    project: Arc<Project>,
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
            project,
        }
    }

    fn path_created_or_updated(&mut self, path: &Path) {
        let mut changes = InstanceChanges::default();

        {
            let imfs = self.imfs.lock().unwrap();
            let root_path = imfs.get_root_for_path(path)
                .expect("Path was outside in-memory filesystem roots");

            let closest_path = self.path_map.descend(root_path, path);
            let &instance_id = self.path_map.get(&closest_path).unwrap();

            trace!("Snapshotting path {}", closest_path.display());

            let snapshot = snapshot_instances_from_imfs(&imfs, &closest_path, &mut self.sync_point_names)
                .expect("Could not generate instance snapshot");

            reconcile_subtree(&mut self.tree, instance_id, &snapshot, &mut self.path_map, &mut self.instance_metadata_map, &mut changes);
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

            // If the path doesn't exist or it's a directory, we don't care if it
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

        let instance_id = match self.path_map.remove(path) {
            Some(id) => id,
            None => {
                // TODO: init.lua files won't be in the path map, that should be
                // patched up!
                trace!("Path was not in path map, ignoring removal.");
                return;
            },
        };

        let removed_subtree = match self.tree.remove_instance(instance_id) {
            Some(tree) => tree,
            None => {
                warn!("Rojo tried to remove an instance that was half cleaned-up. This is probably a bug in Rojo.");
                return;
            },
        };

        let changes = InstanceChanges {
            added: HashSet::new(),
            removed: removed_subtree.iter_all_ids().collect(),
            updated: HashSet::new(),
        };

        trace!("Pushing changes: {}", changes);

        self.message_queue.push_messages(&[changes]);
    }

    pub fn path_renamed(&mut self, from_path: &Path, to_path: &Path) {
        info!("Path renamed from {} to {}", from_path.display(), to_path.display());
        self.path_removed(from_path);
        self.path_created(to_path);
    }

    pub fn get_tree(&self) -> &RbxTree {
        &self.tree
    }

    pub fn get_instance_metadata_map(&self) -> &HashMap<RbxId, InstanceProjectNodeMetadata> {
        &self.instance_metadata_map
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
            construct_instance_node(imfs, instance_name, node, sync_point_names)
        },
        ProjectNode::SyncPoint(node) => {
            let mut snapshot = snapshot_instances_from_imfs(imfs, &node.path, sync_point_names)
                .expect("Could not reify nodes from Imfs");

            snapshot.name = Cow::Borrowed(instance_name);
            sync_point_names.insert(node.path.clone(), instance_name.to_string());

            snapshot
        },
    }
}

fn construct_instance_node<'a>(
    imfs: &'a Imfs,
    instance_name: &'a str,
    project_node: &'a InstanceProjectNode,
    sync_point_names: &mut HashMap<PathBuf, String>,
) -> RbxSnapshotInstance<'a> {
    let mut children = Vec::new();

    for (child_name, child_project_node) in &project_node.children {
        children.push(construct_project_node(imfs, child_name, child_project_node, sync_point_names));
    }

    RbxSnapshotInstance {
        class_name: Cow::Borrowed(&project_node.class_name),
        name: Cow::Borrowed(instance_name),
        properties: HashMap::new(),
        children,
        source_path: None,
        metadata: Some(project_node.metadata.clone()),
    }
}

#[derive(Debug, Clone, Copy)]
enum FileType {
    ModuleScript,
    ServerScript,
    ClientScript,
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
    } else {
        None
    }
}

fn snapshot_instances_from_imfs<'a>(
    imfs: &'a Imfs,
    imfs_path: &Path,
    sync_point_names: &HashMap<PathBuf, String>,
) -> Option<RbxSnapshotInstance<'a>> {
    match imfs.get(imfs_path)? {
        ImfsItem::File(file) => {
            let (instance_name, file_type) = classify_file(file)?;

            let instance_name = if let Some(actual_name) = sync_point_names.get(imfs_path) {
                Cow::Owned(actual_name.clone())
            } else {
                Cow::Borrowed(instance_name)
            };

            let class_name = match file_type {
                FileType::ModuleScript => "ModuleScript",
                FileType::ServerScript => "Script",
                FileType::ClientScript => "LocalScript",
            };

            let contents = str::from_utf8(&file.contents)
                .expect("File did not contain UTF-8 data, which is required for scripts.");

            let mut properties = HashMap::new();
            properties.insert(String::from("Source"), RbxValue::String {
                value: contents.to_string(),
            });

            Some(RbxSnapshotInstance {
                name: instance_name,
                class_name: Cow::Borrowed(class_name),
                properties,
                children: Vec::new(),
                source_path: Some(file.path.clone()),
                metadata: None,
            })
        },
        ImfsItem::Directory(directory) => {
            let init_path = directory.path.join("init.lua");

            let mut instance = if directory.children.contains(&init_path) {
                snapshot_instances_from_imfs(imfs, &init_path, sync_point_names)?
            } else {
                RbxSnapshotInstance {
                    class_name: Cow::Borrowed("Folder"),
                    name: Cow::Borrowed(""), // Assigned later in the method
                    properties: HashMap::new(),
                    children: Vec::new(),
                    source_path: Some(directory.path.clone()),
                    metadata: None,
                }
            };

            instance.name = if let Some(actual_name) = sync_point_names.get(imfs_path) {
                Cow::Owned(actual_name.clone())
            } else {
                Cow::Borrowed(directory.path.file_name()?.to_str()?)
            };

            for child_path in &directory.children {
                if child_path != &init_path {
                    instance.children.push(snapshot_instances_from_imfs(imfs, child_path, sync_point_names)?);
                }
            }

            Some(instance)
        },
    }
}