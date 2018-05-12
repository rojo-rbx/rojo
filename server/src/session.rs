use std::collections::HashMap;
use std::sync::{mpsc, Arc, RwLock};
use std::thread;

use partition::Partition;
use rbx_session::RbxSession;
use vfs_session::VfsSession;
use partition_watcher::PartitionWatcher;

#[derive(Debug, Clone, PartialEq)]
pub struct SessionConfig {
    pub partitions: HashMap<String, Partition>,
}

/// Stub trait for middleware
trait Middleware {
}

pub struct Session {
    config: SessionConfig,
    vfs_session: Arc<RwLock<VfsSession>>,
    rbx_session: Arc<RwLock<RbxSession>>,
    middlewares: Vec<Box<Middleware>>,
    watchers: Vec<PartitionWatcher>,
}

impl Session {
    pub fn new(config: SessionConfig) -> Session {
        let vfs_session = Arc::new(RwLock::new(VfsSession::new(config.clone())));
        let rbx_session = Arc::new(RwLock::new(RbxSession::new(config.clone(), vfs_session.clone())));

        Session {
            vfs_session,
            rbx_session,
            middlewares: Vec::new(),
            watchers: Vec::new(),
            config,
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

        for partition in self.config.partitions.values() {
            let watcher = PartitionWatcher::start_new(partition.clone(), tx.clone());

            self.watchers.push(watcher);
        }

        {
            let vfs_session = self.vfs_session.clone();
            thread::spawn(move || {
                loop {
                    match rx.recv() {
                        Ok(change) => {
                            let mut vfs_session = vfs_session.write().unwrap();
                            vfs_session.handle_change(change);
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
}
