use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    str,
};

use rbx_tree::{RbxTree, RbxId, RbxInstance, RbxValue};

use crate::{
    project::{Project, ProjectNode, InstanceProjectNode},
    message_queue::{Message, MessageQueue},
    imfs::{Imfs, ImfsItem},
};

#[derive(Debug)]
struct PathIdNode {
    id: RbxId,
    children: HashSet<PathBuf>,
}

/// A map from paths to instance IDs, with a bit of additional data that enables
/// removing a path and all of its child paths from the tree in constant time.
#[derive(Debug)]
struct PathIdTree {
    nodes: HashMap<PathBuf, PathIdNode>,
}

impl PathIdTree {
    pub fn new() -> PathIdTree {
        PathIdTree {
            nodes: HashMap::new(),
        }
    }

    pub fn insert(&mut self, path: &Path, id: RbxId) {
        if let Some(parent_path) = path.parent() {
            if let Some(parent) = self.nodes.get_mut(parent_path) {
                parent.children.insert(path.to_path_buf());
            }
        }

        self.nodes.insert(path.to_path_buf(), PathIdNode {
            id,
            children: HashSet::new(),
        });
    }

    pub fn remove(&mut self, root_path: &Path) -> Option<RbxId> {
        if let Some(parent_path) = root_path.parent() {
            if let Some(parent) = self.nodes.get_mut(parent_path) {
                parent.children.remove(root_path);
            }
        }

        let mut root_node = match self.nodes.remove(root_path) {
            Some(node) => node,
            None => return None,
        };

        let root_id = root_node.id;
        let mut to_visit: Vec<PathBuf> = root_node.children.drain().collect();

        loop {
            let next_path = match to_visit.pop() {
                Some(path) => path,
                None => break,
            };

            match self.nodes.remove(&next_path) {
                Some(mut node) => {
                    for child in node.children.drain() {
                        to_visit.push(child);
                    }
                },
                None => {
                    warn!("Consistency issue; tried to remove {} but it was already removed", next_path.display());
                },
            }
        }

        Some(root_id)
    }
}

pub struct RbxSession {
    tree: RbxTree,
    path_id_tree: PathIdTree,
    ids_to_project_paths: HashMap<RbxId, String>,
    message_queue: Arc<MessageQueue>,
    imfs: Arc<Mutex<Imfs>>,
    project: Arc<Project>,
}

impl RbxSession {
    pub fn new(project: Arc<Project>, imfs: Arc<Mutex<Imfs>>, message_queue: Arc<MessageQueue>) -> RbxSession {
        let (tree, path_id_tree, ids_to_project_paths) = {
            let temp_imfs = imfs.lock().unwrap();
            construct_initial_tree(&project, &temp_imfs)
        };

        RbxSession {
            tree,
            path_id_tree,
            ids_to_project_paths,
            message_queue,
            imfs,
            project,
        }
    }

    pub fn path_created(&mut self, path: &Path) {
        info!("Path created: {}", path.display());
    }

    pub fn path_updated(&mut self, path: &Path) {
        info!("Path updated: {}", path.display());
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

struct ConstructContext<'a> {
    tree: RbxTree,
    imfs: &'a Imfs,
    path_id_tree: PathIdTree,
    ids_to_project_paths: HashMap<RbxId, String>,
}

fn construct_initial_tree(
    project: &Project,
    imfs: &Imfs,
) -> (RbxTree, PathIdTree, HashMap<RbxId, String>) {
    let path_id_tree = PathIdTree::new();
    let ids_to_project_paths = HashMap::new();
    let tree = RbxTree::new(RbxInstance {
        name: "this isn't supposed to be here".to_string(),
        class_name: "ahhh, help me".to_string(),
        properties: HashMap::new(),
    });

    let root_id = tree.get_root_id();

    let mut context = ConstructContext {
        tree,
        imfs,
        path_id_tree,
        ids_to_project_paths,
    };

    construct_project_node(
        &mut context,
        root_id,
        "<<<ROOT>>>".to_string(),
        "<<<ROOT>>>",
        &project.tree,
    );

    (context.tree, context.path_id_tree, context.ids_to_project_paths)
}

fn construct_project_node(
    context: &mut ConstructContext,
    parent_instance_id: RbxId,
    instance_path: String,
    instance_name: &str,
    project_node: &ProjectNode,
) {
    match project_node {
        ProjectNode::Instance(node) => {
            let id = construct_instance_node(context, parent_instance_id, &instance_path, instance_name, node);
            context.ids_to_project_paths.insert(id, instance_path.to_string());
        },
        ProjectNode::SyncPoint(node) => {
            let id = construct_sync_point_node(context, parent_instance_id, instance_name, &node.path);
            context.ids_to_project_paths.insert(id, instance_path.to_string());
        },
    }
}

fn construct_instance_node(
    context: &mut ConstructContext,
    parent_instance_id: RbxId,
    instance_path: &str,
    instance_name: &str,
    project_node: &InstanceProjectNode,
) -> RbxId {
    let instance = RbxInstance {
        class_name: project_node.class_name.clone(),
        name: instance_name.to_string(),
        properties: HashMap::new(),
    };

    let id = context.tree.insert_instance(instance, parent_instance_id);

    for (child_name, child_project_node) in &project_node.children {
        let child_path = format!("{}/{}", instance_path, child_name);
        construct_project_node(context, id, child_path, child_name, child_project_node);
    }

    id
}

fn construct_sync_point_node(
    context: &mut ConstructContext,
    parent_instance_id: RbxId,
    instance_name: &str,
    file_path: &Path,
) -> RbxId {
    match context.imfs.get(&file_path) {
        Some(ImfsItem::File(file)) => {
            let contents = str::from_utf8(&file.contents).unwrap();

            let mut properties = HashMap::new();
            properties.insert("Source".to_string(), RbxValue::String { value: contents.to_string() });

            let instance = RbxInstance {
                class_name: "ModuleScript".to_string(),
                name: instance_name.to_string(),
                properties,
            };

            let id = context.tree.insert_instance(instance, parent_instance_id);
            context.path_id_tree.insert(&file.path, id);

            id
        },
        Some(ImfsItem::Directory(directory)) => {
            let instance = RbxInstance {
                class_name: "Folder".to_string(),
                name: instance_name.to_string(),
                properties: HashMap::new(),
            };

            let id = context.tree.insert_instance(instance, parent_instance_id);
            context.path_id_tree.insert(&directory.path, id);

            for child_path in &directory.children {
                let child_instance_name = child_path.file_name().unwrap().to_str().unwrap();
                construct_sync_point_node(context, id, child_instance_name, child_path);
            }

            id
        },
        None => panic!("Couldn't read {} from disk", file_path.display()),
    }
}