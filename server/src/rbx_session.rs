use std::{
    borrow::Cow,
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
    str,
};

use rbx_tree::{RbxTree, RbxId};

use crate::{
    project::{Project, ProjectNode, InstanceProjectNode},
    message_queue::{Message, MessageQueue},
    imfs::{Imfs, ImfsItem, ImfsFile},
    path_map::PathMap,
    rbx_snapshot::{RbxSnapshotInstance, RbxSnapshotValue, reify_root},
};

pub struct RbxSession {
    tree: RbxTree,
    path_id_tree: PathMap<RbxId>,
    ids_to_project_paths: HashMap<RbxId, String>,
    message_queue: Arc<MessageQueue>,
    imfs: Arc<Mutex<Imfs>>,
    project: Arc<Project>,
}

impl RbxSession {
    pub fn new(project: Arc<Project>, imfs: Arc<Mutex<Imfs>>, message_queue: Arc<MessageQueue>) -> RbxSession {
        let tree = {
            let temp_imfs = imfs.lock().unwrap();
            construct_initial_tree(&project, &temp_imfs)
        };

        // TODO: Restore these?
        let path_id_tree = PathMap::new();
        let ids_to_project_paths = HashMap::new();

        RbxSession {
            tree,
            path_id_tree,
            ids_to_project_paths,
            message_queue,
            imfs,
            project,
        }
    }

    fn path_created_or_updated(&mut self, path: &Path) {
        if let Some(instance_id) = self.path_id_tree.get(path) {
            // TODO: Replace instance with ID `instance_id` with new instance
        }

        // TODO: Crawl up path to find first node represented in the tree or a
        // sync point root. That path immediately before we find an existing
        // node should be read into the tree.
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
                Some(ImfsItem::Directory(_)) | None => return,
                Some(ImfsItem::File(_)) => {},
            }
        }

        self.path_created_or_updated(path);
    }

    pub fn path_removed(&mut self, path: &Path) {
        info!("Path removed: {}", path.display());

        let instance_id = match self.path_id_tree.remove(path) {
            Some(id) => id,
            None => return,
        };

        let removed_subtree = match self.tree.remove_instance(instance_id) {
            Some(tree) => tree,
            None => {
                warn!("Rojo tried to remove an instance that was half cleaned-up. This is probably a bug in Rojo.");
                return;
            },
        };

        let removed_ids: Vec<RbxId> = removed_subtree.iter_all_ids().collect();

        self.message_queue.push_messages(&[
            Message::InstancesRemoved {
                ids: removed_ids,
            },
        ]);
    }

    pub fn path_renamed(&mut self, from_path: &Path, to_path: &Path) {
        info!("Path renamed from {} to {}", from_path.display(), to_path.display());
        self.path_removed(from_path);
        self.path_created(to_path);
    }

    pub fn get_tree(&self) -> &RbxTree {
        &self.tree
    }

    pub fn get_project_path_map(&self) -> &HashMap<RbxId, String> {
        &self.ids_to_project_paths
    }
}

pub fn construct_oneoff_tree(project: &Project, imfs: &Imfs) -> RbxTree {
    construct_initial_tree(project, imfs)
}

struct ConstructContext<'a> {
    imfs: &'a Imfs,
    path_id_tree: PathMap<RbxId>,
    ids_to_project_paths: HashMap<RbxId, String>,
}

fn construct_initial_tree(
    project: &Project,
    imfs: &Imfs,
) -> RbxTree {
    let path_id_tree = PathMap::new();
    let ids_to_project_paths = HashMap::new();

    let mut context = ConstructContext {
        imfs,
        path_id_tree,
        ids_to_project_paths,
    };

    let snapshot = construct_project_node(
        &mut context,
        "",
        &project.name,
        &project.tree,
    );

    reify_root(&snapshot)
}

fn construct_project_node<'a>(
    context: &mut ConstructContext<'a>,
    instance_path: &str,
    instance_name: &'a str,
    project_node: &'a ProjectNode,
) -> RbxSnapshotInstance<'a> {
    match project_node {
        ProjectNode::Instance(node) => {
            construct_instance_node(context, &instance_path, instance_name, node)
        },
        ProjectNode::SyncPoint(node) => {
            construct_sync_point_node(context, instance_name, &node.path)
        },
    }
}

fn construct_instance_node<'a>(
    context: &mut ConstructContext<'a>,
    instance_path: &str,
    instance_name: &'a str,
    project_node: &'a InstanceProjectNode,
) -> RbxSnapshotInstance<'a> {
    let mut children = Vec::new();

    for (child_name, child_project_node) in &project_node.children {
        let child_path = if instance_path.is_empty() {
            child_name.clone()
        } else {
            format!("{}/{}", instance_path, child_name)
        };

        children.push(construct_project_node(context, &child_path, child_name, child_project_node));
    }

    RbxSnapshotInstance {
        class_name: Cow::Borrowed(&project_node.class_name),
        name: Cow::Borrowed(instance_name),
        properties: HashMap::new(),
        children,
    }
}

#[derive(Debug, Clone, Copy)]
enum FileType {
    ModuleScript,
    ServerScript,
    ClientScript,
}

fn classify_file(file: &ImfsFile) -> Option<FileType> {
    let file_name = file.path.file_name()?.to_str()?;

    if file_name.ends_with(".server.lua") {
        Some(FileType::ServerScript)
    } else if file_name.ends_with(".client.lua") {
        Some(FileType::ClientScript)
    } else if file_name.ends_with(".lua") {
        Some(FileType::ModuleScript)
    } else {
        None
    }
}

fn construct_sync_point_node<'a>(
    context: &mut ConstructContext<'a>,
    instance_name: &'a str,
    file_path: &Path,
) -> RbxSnapshotInstance<'a> {
    match context.imfs.get(&file_path) {
        Some(ImfsItem::File(file)) => {
            let file_type = classify_file(file).unwrap(); // TODO: Don't die here!

            let class_name = match file_type {
                FileType::ModuleScript => "ModuleScript",
                FileType::ServerScript => "Script",
                FileType::ClientScript => "LocalScript",
            };

            let contents = str::from_utf8(&file.contents).unwrap();

            let mut properties = HashMap::new();
            properties.insert("Source".to_string(), RbxSnapshotValue::String(Cow::Borrowed(contents)));

            let instance = RbxSnapshotInstance {
                class_name: Cow::Borrowed(class_name),
                name: Cow::Borrowed(instance_name),
                properties,
                children: Vec::new(),
            };

            instance
        },
        Some(ImfsItem::Directory(directory)) => {
            let init_path = directory.path.join("init.lua");

            let mut instance = if directory.children.contains(&init_path) {
                construct_sync_point_node(context, instance_name, &init_path)
            } else {
                RbxSnapshotInstance {
                    class_name: Cow::Borrowed("Folder"),
                    name: Cow::Borrowed(instance_name),
                    properties: HashMap::new(),
                    children: Vec::new(),
                }
            };

            for child_path in &directory.children {
                if child_path != &init_path {
                    let child_instance_name = match context.imfs.get(child_path).unwrap() {
                        ImfsItem::File(_) => child_path.file_stem().unwrap().to_str().unwrap(),
                        ImfsItem::Directory(_) => child_path.file_name().unwrap().to_str().unwrap(),
                    };

                    instance.children.push(construct_sync_point_node(context, child_instance_name, child_path));
                }
            }

            instance
        },
        None => panic!("Couldn't read {} from disk", file_path.display()),
    }
}