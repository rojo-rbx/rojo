use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    str,
};

use rbx_tree::{RbxTree, RbxId, RbxInstance, RbxValue};

use crate::{
    project::{Project, ProjectNode, InstanceProjectNode},
    message_queue::MessageQueue,
    vfs::{Vfs, VfsItem},
};

pub struct RbxSession {
    tree: RbxTree,
    paths_to_ids: HashMap<PathBuf, RbxId>,
    message_queue: Arc<MessageQueue>,
    vfs: Arc<Mutex<Vfs>>,
    project: Arc<Project>,
}

impl RbxSession {
    pub fn new(project: Arc<Project>, vfs: Arc<Mutex<Vfs>>, message_queue: Arc<MessageQueue>) -> RbxSession {
        let (tree, paths_to_ids) = {
            let temp_vfs = vfs.lock().unwrap();
            construct_initial_tree(&project, &temp_vfs)
        };

        {
            use serde_json;
            println!("{}", serde_json::to_string(&tree).unwrap());
        }

        RbxSession {
            tree,
            paths_to_ids,
            message_queue,
            vfs,
            project,
        }
    }

    pub fn path_created_or_updated(&mut self, path: &Path) {
        println!("Path changed: {}", path.display());
    }

    pub fn path_removed(&mut self, path: &Path) {
        println!("Path removed: {}", path.display());
    }

    pub fn path_renamed(&mut self, from_path: &Path, to_path: &Path) {
        println!("Path renamed from {} to {}", from_path.display(), to_path.display());
    }

    pub fn get_tree(&self) -> &RbxTree {
        &self.tree
    }
}

fn construct_initial_tree(
    project: &Project,
    vfs: &Vfs,
) -> (RbxTree, HashMap<PathBuf, RbxId>) {
    let mut paths_to_ids = HashMap::new();
    let mut tree = RbxTree::new(RbxInstance {
        name: "this isn't supposed to be here".to_string(),
        class_name: "ahhh, help me".to_string(),
        properties: HashMap::new(),
    });

    let root_id = tree.get_root_id();

    construct_initial_tree_node(&mut tree, vfs, &mut paths_to_ids, root_id, "<<<ROOT>>>", &project.tree);

    (tree, paths_to_ids)
}

fn construct_initial_tree_node(
    tree: &mut RbxTree,
    vfs: &Vfs,
    paths_to_ids: &mut HashMap<PathBuf, RbxId>,
    parent_instance_id: RbxId,
    instance_name: &str,
    project_node: &ProjectNode,
) {
    match project_node {
        ProjectNode::Instance(node) => {
            construct_instance_node(tree, vfs, paths_to_ids, parent_instance_id, instance_name, node);
        },
        ProjectNode::SyncPoint(node) => {
            construct_sync_point_node(tree, vfs, paths_to_ids, parent_instance_id, instance_name, &node.path);
        },
    }
}

fn construct_instance_node(
    tree: &mut RbxTree,
    vfs: &Vfs,
    paths_to_ids: &mut HashMap<PathBuf, RbxId>,
    parent_instance_id: RbxId,
    instance_name: &str,
    project_node: &InstanceProjectNode,
) {
    let instance = RbxInstance {
        class_name: project_node.class_name.clone(),
        name: instance_name.to_string(),
        properties: HashMap::new(),
    };

    let id = tree.insert_instance(instance, parent_instance_id);

    for (child_name, child_project_node) in &project_node.children {
        construct_initial_tree_node(tree, vfs, paths_to_ids, id, child_name, child_project_node);
    }
}

fn construct_sync_point_node(
    tree: &mut RbxTree,
    vfs: &Vfs,
    paths_to_ids: &mut HashMap<PathBuf, RbxId>,
    parent_instance_id: RbxId,
    instance_name: &str,
    file_path: &Path,
) {
    match vfs.get(&file_path) {
        Some(VfsItem::File(file)) => {
            let contents = str::from_utf8(vfs.get_contents(&file.path).unwrap()).unwrap();

            let mut properties = HashMap::new();
            properties.insert("Source".to_string(), RbxValue::String { value: contents.to_string() });

            let instance = RbxInstance {
                class_name: "ModuleScript".to_string(),
                name: instance_name.to_string(),
                properties,
            };

            let id = tree.insert_instance(instance, parent_instance_id);
            paths_to_ids.insert(file.path.clone(), id);
        },
        Some(VfsItem::Directory(directory)) => {
            let instance = RbxInstance {
                class_name: "Folder".to_string(),
                name: instance_name.to_string(),
                properties: HashMap::new(),
            };

            let id = tree.insert_instance(instance, parent_instance_id);
            paths_to_ids.insert(directory.path.clone(), id);

            for child_path in &directory.children {
                let child_instance_name = child_path.file_name().unwrap().to_str().unwrap();
                construct_sync_point_node(tree, vfs, paths_to_ids, id, child_instance_name, child_path);
            }
        },
        None => panic!("Couldn't read {} from disk", file_path.display()),
    }
}