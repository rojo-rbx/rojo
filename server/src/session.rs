use std::{
    collections::HashMap,
    sync::{Arc, RwLock, Mutex, mpsc},
    path::{Path, PathBuf},
    thread,
    io,
    time::Duration,
    str,
};

use serde_json;

use notify::{
    self,
    DebouncedEvent,
    RecommendedWatcher,
    RecursiveMode,
    Watcher,
};

use rbx_tree::{RbxId, RbxTree, RbxInstance, RbxValue};

use crate::{
    message_queue::MessageQueue,
    project::{Project, ProjectNode},
    vfs::{Vfs, VfsItem},
    session_id::SessionId,
};

const WATCH_TIMEOUT_MS: u64 = 100;

pub struct Session {
    project: Project,
    pub session_id: SessionId,
    pub message_queue: Arc<MessageQueue>,
    pub tree: Arc<RwLock<RbxTree>>,
    paths_to_ids: HashMap<PathBuf, RbxId>,
    vfs: Arc<Mutex<Vfs>>,
    watchers: Vec<RecommendedWatcher>,
}

fn add_sync_points(vfs: &mut Vfs, project_node: &ProjectNode) -> io::Result<()> {
    match project_node {
        ProjectNode::Regular { children, .. } => {
            for child in children.values() {
                add_sync_points(vfs, child)?;
            }
        },
        ProjectNode::SyncPoint { path } => {
            vfs.add_root(path)?;
        },
    }

    Ok(())
}

fn read_sync_to_rbx(
    tree: &mut RbxTree,
    vfs: &Vfs,
    paths_to_ids: &mut HashMap<PathBuf, RbxId>,
    parent_node_id: RbxId,
    project_node_name: &str,
    path: &Path
) {
    match vfs.get(path) {
        Some(VfsItem::File(file)) => {
            let contents = str::from_utf8(vfs.get_contents(&file.path).unwrap()).unwrap();

            let mut properties = HashMap::new();
            properties.insert("Source".to_string(), RbxValue::String { value: contents.to_string() });

            let instance = RbxInstance {
                class_name: "ModuleScript".to_string(),
                name: project_node_name.to_string(),
                properties,
            };

            let id = tree.insert_instance(instance, parent_node_id);
            paths_to_ids.insert(path.to_path_buf(), id);
        },
        Some(VfsItem::Directory(directory)) => {
            let instance = RbxInstance {
                class_name: "Folder".to_string(),
                name: project_node_name.to_string(),
                properties: HashMap::new(),
            };

            let id = tree.insert_instance(instance, parent_node_id);
            paths_to_ids.insert(path.to_path_buf(), id);

            for child_path in &directory.children {
                let child_name = child_path.file_name().unwrap().to_str().unwrap();
                read_sync_to_rbx(tree, vfs, paths_to_ids, id, child_name, child_path);
            }
        },
        None => panic!("Couldn't read {} from disk", path.display()),
    }
}

fn read_to_rbx(
    tree: &mut RbxTree,
    vfs: &Vfs,
    paths_to_ids: &mut HashMap<PathBuf, RbxId>,
    parent_node_id: RbxId,
    project_node_name: &str,
    project_node: &ProjectNode
) {
    match project_node {
        ProjectNode::Regular { children, class_name, .. } => {
            let instance = RbxInstance {
                class_name: class_name.clone(),
                name: project_node_name.to_string(),
                properties: HashMap::new(),
            };

            let id = tree.insert_instance(instance, parent_node_id);

            for (child_name, child_project_node) in children {
                read_to_rbx(tree, vfs, paths_to_ids, id, child_name, child_project_node);
            }
        },
        ProjectNode::SyncPoint { path } => {
            read_sync_to_rbx(tree, vfs, paths_to_ids, parent_node_id, project_node_name, path);
        },
    }
}

impl Session {
    pub fn new(project: Project) -> io::Result<Session> {
        let mut vfs = Vfs::new();

        let (change_tx, change_rx) = mpsc::channel();

        add_sync_points(&mut vfs, &project.tree)
            .expect("Could not add sync points when starting new Rojo session");

        let mut tree = RbxTree::new(RbxInstance {
            name: "ahhhh".to_string(),
            class_name: "ahhh help me".to_string(),
            properties: HashMap::new(),
        });

        let mut paths_to_ids = HashMap::new();

        let root_id = tree.get_root_id();
        read_to_rbx(&mut tree, &vfs, &mut paths_to_ids, root_id, "root", &project.tree);

        println!("tree:\n{}", serde_json::to_string(&tree).unwrap());

        let vfs = Arc::new(Mutex::new(vfs));
        let mut watchers = Vec::new();

        {
            let vfs_temp = vfs.lock().unwrap();

            for root in vfs_temp.get_roots() {
                let (watch_tx, watch_rx) = mpsc::channel();

                let mut watcher = notify::watcher(watch_tx, Duration::from_millis(WATCH_TIMEOUT_MS)).unwrap();

                watcher.watch(root, RecursiveMode::Recursive)
                    .expect("Could not watch directory");

                watchers.push(watcher);

                let change_tx = change_tx.clone();
                let vfs = Arc::clone(&vfs);

                thread::spawn(move || {
                    loop {
                        match watch_rx.recv() {
                            Ok(event) => {
                                match event {
                                    DebouncedEvent::Create(path) | DebouncedEvent::Write(path) => {
                                        let mut vfs = vfs.lock().unwrap();
                                        vfs.add_or_update(&path).unwrap();
                                        change_tx.send(path.clone()).unwrap();
                                    },
                                    DebouncedEvent::Remove(path) => {
                                        let mut vfs = vfs.lock().unwrap();
                                        vfs.remove(&path);
                                        change_tx.send(path.clone()).unwrap();
                                    },
                                    DebouncedEvent::Rename(from_path, to_path) => {
                                        let mut vfs = vfs.lock().unwrap();
                                        vfs.remove(&from_path);
                                        vfs.add_or_update(&to_path).unwrap();
                                        change_tx.send(from_path.clone()).unwrap();
                                        change_tx.send(to_path.clone()).unwrap();
                                    },
                                    _ => continue,
                                };
                            },
                            Err(_) => break,
                        };
                    }
                    info!("Watcher thread stopped");
                });
            }
        }

        let message_queue = Arc::new(MessageQueue::new());
        let session_id = SessionId::new();

        Ok(Session {
            session_id,
            paths_to_ids,
            project,
            message_queue,
            tree: Arc::new(RwLock::new(tree)),
            vfs,
            watchers: Vec::new(),
        })
    }

    pub fn get_project(&self) -> &Project {
        &self.project
    }
}