use std::{
    sync::{Arc, RwLock, Mutex, mpsc},
    thread,
    io,
    time::Duration,
};

use rand;

use notify::{
    self,
    DebouncedEvent,
    RecommendedWatcher,
    RecursiveMode,
    Watcher,
};

use rbx_tree::RbxTree;

use ::{
    message_queue::MessageQueue,
    project::{Project, ProjectNode},
    vfs::Vfs,
};

const WATCH_TIMEOUT_MS: u64 = 100;

pub struct Session {
    project: Project,
    pub session_id: String,
    pub message_queue: Arc<MessageQueue>,
    pub tree: Arc<RwLock<RbxTree>>,
    vfs: Arc<Mutex<Vfs>>,
    watchers: Vec<RecommendedWatcher>,
}

impl Session {
    pub fn new(project: Project) -> Session {
        let session_id = rand::random::<u64>().to_string();

        Session {
            session_id,
            project,
            message_queue: Arc::new(MessageQueue::new()),
            tree: Arc::new(RwLock::new(RbxTree::new())),
            vfs: Arc::new(Mutex::new(Vfs::new())),
            watchers: Vec::new(),
        }
    }

    pub fn start(&mut self) -> io::Result<()> {
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

        {
            let mut vfs = self.vfs.lock().unwrap();

            for child in self.project.tree.values() {
                add_sync_points(&mut vfs, child)?;
            }

            for root in vfs.get_roots() {
                info!("Watching {}", root.display());

                let (watch_tx, watch_rx) = mpsc::channel();

                let mut watcher = notify::watcher(watch_tx, Duration::from_millis(WATCH_TIMEOUT_MS)).unwrap();

                watcher.watch(root, RecursiveMode::Recursive).unwrap();
                self.watchers.push(watcher);

                let vfs = Arc::clone(&self.vfs);

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

        Ok(())
    }

    pub fn get_project(&self) -> &Project {
        &self.project
    }
}