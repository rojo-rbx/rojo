use std::{
    collections::HashMap,
    sync::{Arc, RwLock, Mutex, mpsc},
    thread,
    io,
    time::Duration,
};

use notify::{
    self,
    DebouncedEvent,
    RecommendedWatcher,
    RecursiveMode,
    Watcher,
};

use rbx_tree::{RbxTree, RbxInstance};

use crate::{
    message_queue::MessageQueue,
    project::{Project, ProjectNode},
    vfs::Vfs,
    session_id::SessionId,
};

const WATCH_TIMEOUT_MS: u64 = 100;

pub struct Session {
    project: Project,
    pub session_id: SessionId,
    pub message_queue: Arc<MessageQueue>,
    pub tree: Arc<RwLock<RbxTree>>,
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

impl Session {
    pub fn new(project: Project) -> io::Result<Session> {
        let session_id = SessionId::new();
        let vfs = Arc::new(Mutex::new(Vfs::new()));
        let message_queue = Arc::new(MessageQueue::new());
        let mut watchers = Vec::new();

        {
            let mut vfs_temp = vfs.lock().unwrap();

            add_sync_points(&mut vfs_temp, &project.tree)
                .expect("Could not add sync points when starting new Rojo session");

            for root in vfs_temp.get_roots() {
                let (watch_tx, watch_rx) = mpsc::channel();

                let mut watcher = notify::watcher(watch_tx, Duration::from_millis(WATCH_TIMEOUT_MS)).unwrap();

                watcher.watch(root, RecursiveMode::Recursive)
                    .expect("Could not watch directory");

                watchers.push(watcher);

                let vfs = Arc::clone(&vfs);

                thread::spawn(move || {
                    loop {
                        match watch_rx.recv() {
                            Ok(event) => {
                                match event {
                                    DebouncedEvent::Create(path) | DebouncedEvent::Write(path) => {
                                        let mut vfs = vfs.lock().unwrap();
                                        vfs.add_or_update(&path).unwrap();
                                    },
                                    DebouncedEvent::Remove(path) => {
                                        let mut vfs = vfs.lock().unwrap();
                                        vfs.remove(&path);
                                    },
                                    DebouncedEvent::Rename(from_path, to_path) => {
                                        let mut vfs = vfs.lock().unwrap();
                                        vfs.remove(&from_path);
                                        vfs.add_or_update(&to_path).unwrap();
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

        let tree = RbxTree::new(RbxInstance {
            name: "ahhhh".to_string(),
            class_name: "ahhh help me".to_string(),
            properties: HashMap::new(),
        });

        Ok(Session {
            session_id,
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