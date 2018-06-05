use std::sync::{mpsc, Arc, RwLock};
use std::thread;

use message_session::MessageSession;
use partition_watcher::PartitionWatcher;
use project::Project;
use rbx_session::RbxSession;
use vfs_session::VfsSession;

/// Stub trait for middleware
trait Middleware {
}

pub struct Session {
    project: Project,
    vfs_session: Arc<RwLock<VfsSession>>,
    rbx_session: Arc<RwLock<RbxSession>>,
    message_session: MessageSession,
    watchers: Vec<PartitionWatcher>,
}

impl Session {
    pub fn new(project: Project) -> Session {
        let message_session = MessageSession::new();
        let vfs_session = Arc::new(RwLock::new(VfsSession::new(project.clone())));
        let rbx_session = Arc::new(RwLock::new(RbxSession::new(project.clone(), vfs_session.clone(), message_session.clone())));

        Session {
            vfs_session,
            rbx_session,
            watchers: Vec::new(),
            message_session,
            project,
        }
    }

    pub fn start(&mut self) {
        {
            let mut vfs_session = self.vfs_session.write().unwrap();
            vfs_session.read_partitions();
        }

        {
            let mut rbx_session = self.rbx_session.write().unwrap();
            rbx_session.read_partitions();
        }

        let (tx, rx) = mpsc::channel();

        for partition in self.project.partitions.values() {
            let watcher = PartitionWatcher::start_new(partition.clone(), tx.clone());

            self.watchers.push(watcher);
        }

        {
            let vfs_session = self.vfs_session.clone();
            let rbx_session = self.rbx_session.clone();

            thread::spawn(move || {
                loop {
                    match rx.recv() {
                        Ok(change) => {
                            {
                                let mut vfs_session = vfs_session.write().unwrap();
                                vfs_session.handle_change(&change);
                            }

                            {
                                let mut rbx_session = rbx_session.write().unwrap();
                                rbx_session.handle_change(&change);
                            }
                        },
                        Err(_) => break,
                    }
                }
            });
        }
    }

    pub fn stop(self) {
    }

    pub fn get_vfs_session(&self) -> Arc<RwLock<VfsSession>> {
        self.vfs_session.clone()
    }

    pub fn get_rbx_session(&self) -> Arc<RwLock<RbxSession>> {
        self.rbx_session.clone()
    }

    pub fn get_message_session(&self) -> MessageSession {
        self.message_session.clone()
    }
}
