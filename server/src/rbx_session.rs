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
    imfs::{Imfs, ImfsItem},
};

pub struct RbxSession {
    tree: RbxTree,
    paths_to_ids: HashMap<PathBuf, RbxId>,
    ids_to_project_paths: HashMap<RbxId, String>,
    message_queue: Arc<MessageQueue>,
    imfs: Arc<Mutex<Imfs>>,
    project: Arc<Project>,
}

impl RbxSession {
    pub fn new(project: Arc<Project>, imfs: Arc<Mutex<Imfs>>, message_queue: Arc<MessageQueue>) -> RbxSession {
        let (tree, paths_to_ids, ids_to_project_paths) = {
            let temp_imfs = imfs.lock().unwrap();
            construct_initial_tree(&project, &temp_imfs)
        };

        RbxSession {
            tree,
            paths_to_ids,
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
    }

    pub fn path_renamed(&mut self, from_path: &Path, to_path: &Path) {
        info!("Path renamed from {} to {}", from_path.display(), to_path.display());
    }

    pub fn get_tree(&self) -> &RbxTree {
        &self.tree
    }

    pub fn get_project_path_map(&self) -> &HashMap<RbxId, String> {
        &self.ids_to_project_paths
    }
}

fn construct_initial_tree(
    project: &Project,
    imfs: &Imfs,
) -> (RbxTree, HashMap<PathBuf, RbxId>, HashMap<RbxId, String>) {
    let paths_to_ids = HashMap::new();
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
        paths_to_ids,
        ids_to_project_paths,
    };

    construct_project_node(
        &mut context,
        root_id,
        "<<<ROOT>>>".to_string(),
        "<<<ROOT>>>",
        &project.tree,
    );

    (context.tree, context.paths_to_ids, context.ids_to_project_paths)
}

struct ConstructContext<'a> {
    tree: RbxTree,
    imfs: &'a Imfs,
    paths_to_ids: HashMap<PathBuf, RbxId>,
    ids_to_project_paths: HashMap<RbxId, String>,
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
            context.paths_to_ids.insert(file.path.clone(), id);

            id
        },
        Some(ImfsItem::Directory(directory)) => {
            let instance = RbxInstance {
                class_name: "Folder".to_string(),
                name: instance_name.to_string(),
                properties: HashMap::new(),
            };

            let id = context.tree.insert_instance(instance, parent_instance_id);
            context.paths_to_ids.insert(directory.path.clone(), id);

            for child_path in &directory.children {
                let child_instance_name = child_path.file_name().unwrap().to_str().unwrap();
                construct_sync_point_node(context, id, child_instance_name, child_path);
            }

            id
        },
        None => panic!("Couldn't read {} from disk", file_path.display()),
    }
}