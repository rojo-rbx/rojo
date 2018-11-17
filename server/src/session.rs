use std::{
    sync::{Arc, Mutex, mpsc},
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

use crate::{
    message_queue::MessageQueue,
    project::{Project, ProjectNode},
    vfs::Vfs,
    session_id::SessionId,
    rbx_session::RbxSession,
};

const WATCH_TIMEOUT_MS: u64 = 100;

pub struct Session {
    project: Arc<Project>,
    pub session_id: SessionId,
    pub message_queue: Arc<MessageQueue>,
    pub rbx_session: Arc<Mutex<RbxSession>>,
    vfs: Arc<Mutex<Vfs>>,
    watchers: Vec<RecommendedWatcher>,
}

fn add_sync_points(vfs: &mut Vfs, project_node: &ProjectNode) -> io::Result<()> {
    match project_node {
        ProjectNode::Instance(node) => {
            for child in node.children.values() {
                add_sync_points(vfs, child)?;
            }
        },
        ProjectNode::SyncPoint(node) => {
            vfs.add_root(&node.path)?;
        },
    }

    Ok(())
}

impl Session {
    pub fn new(project: Project) -> io::Result<Session> {
        let project = Arc::new(project);
        let message_queue = Arc::new(MessageQueue::new());
        let vfs = Arc::new(Mutex::new(Vfs::new()));

        {
            let mut vfs = vfs.lock().unwrap();
            add_sync_points(&mut vfs, &project.tree)
                .expect("Could not add sync points when starting new Rojo session");
        }

        let rbx_session = Arc::new(Mutex::new(RbxSession::new(
            Arc::clone(&project),
            Arc::clone(&vfs),
            Arc::clone(&message_queue),
        )));

        let mut watchers = Vec::new();

        {
            let vfs_temp = vfs.lock().unwrap();

            for root in vfs_temp.get_roots() {
                info!("Watching path {}", root.display());

                let (watch_tx, watch_rx) = mpsc::channel();

                let mut watcher = notify::watcher(watch_tx, Duration::from_millis(WATCH_TIMEOUT_MS)).unwrap();

                watcher.watch(root, RecursiveMode::Recursive)
                    .expect("Could not watch directory");

                watchers.push(watcher);

                let vfs = Arc::clone(&vfs);
                let rbx_session = Arc::clone(&rbx_session);

                thread::spawn(move || {
                    loop {
                        match watch_rx.recv() {
                            Ok(event) => {
                                match event {
                                    DebouncedEvent::Create(path) => {
                                        {
                                            let mut vfs = vfs.lock().unwrap();
                                            vfs.path_created(&path).unwrap();
                                        }

                                        {
                                            let mut rbx_session = rbx_session.lock().unwrap();
                                            rbx_session.path_created(&path);
                                        }
                                    },
                                    DebouncedEvent::Write(path) => {
                                        {
                                            let mut vfs = vfs.lock().unwrap();
                                            vfs.path_updated(&path).unwrap();
                                        }

                                        {
                                            let mut rbx_session = rbx_session.lock().unwrap();
                                            rbx_session.path_updated(&path);
                                        }
                                    },
                                    DebouncedEvent::Remove(path) => {
                                        {
                                            let mut vfs = vfs.lock().unwrap();
                                            vfs.path_removed(&path).unwrap();
                                        }

                                        {
                                            let mut rbx_session = rbx_session.lock().unwrap();
                                            rbx_session.path_removed(&path);
                                        }
                                    },
                                    DebouncedEvent::Rename(from_path, to_path) => {
                                        {
                                            let mut vfs = vfs.lock().unwrap();
                                            vfs.path_moved(&from_path, &to_path).unwrap();
                                        }

                                        {
                                            let mut rbx_session = rbx_session.lock().unwrap();
                                            rbx_session.path_renamed(&from_path, &to_path);
                                        }
                                    },
                                    _ => {},
                                };
                            },
                            Err(_) => break,
                        };
                    }
                    info!("Watcher thread stopped");
                });
            }
        }

        let session_id = SessionId::new();

        Ok(Session {
            session_id,
            rbx_session,
            project,
            message_queue,
            vfs,
            watchers,
        })
    }

    pub fn get_project(&self) -> &Project {
        &self.project
    }
}