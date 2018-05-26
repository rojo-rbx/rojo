use std::collections::HashMap;
use std::sync::{mpsc, Arc, RwLock, Mutex};
use std::thread;

use partition::Partition;
use rbx_session::RbxSession;
use vfs_session::VfsSession;
use partition_watcher::PartitionWatcher;
use id::Id;

#[derive(Debug, Clone, PartialEq)]
pub struct SessionConfig {
    pub partitions: HashMap<String, Partition>,
}

/// Stub trait for middleware
trait Middleware {
}

#[derive(Debug, Clone, Serialize)]
pub enum SessionEvent {
    Something,
}

pub struct Session {
    config: SessionConfig,
    vfs_session: Arc<RwLock<VfsSession>>,
    rbx_session: Arc<RwLock<RbxSession>>,
    // middlewares: Vec<Box<Middleware>>,
    watchers: Vec<PartitionWatcher>,
    events: Arc<RwLock<Vec<SessionEvent>>>,
    event_listeners: Arc<Mutex<HashMap<Id, mpsc::Sender<()>>>>,
}

impl Session {
    pub fn new(config: SessionConfig) -> Session {
        let vfs_session = Arc::new(RwLock::new(VfsSession::new(config.clone())));
        let rbx_session = Arc::new(RwLock::new(RbxSession::new(config.clone(), vfs_session.clone())));

        Session {
            vfs_session,
            rbx_session,
            // middlewares: Vec::new(),
            watchers: Vec::new(),
            events: Arc::new(RwLock::new(Vec::new())),
            event_listeners: Arc::new(Mutex::new(HashMap::new())),
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
            let event_listeners = self.event_listeners.clone();
            let events = self.events.clone();

            thread::spawn(move || {
                loop {
                    match rx.recv() {
                        Ok(change) => {
                            {
                                let mut vfs_session = vfs_session.write().unwrap();
                                vfs_session.handle_change(change);
                            }

                            // TODO: Handle change in RbxSession

                            {
                                let mut events = events.write().unwrap();
                                events.push(SessionEvent::Something);
                            }

                            {
                                let listeners = event_listeners.lock().unwrap();

                                for listener in listeners.values() {
                                    listener.send(()).unwrap();
                                }
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

    pub fn get_events(&self) -> Arc<RwLock<Vec<SessionEvent>>> {
        self.events.clone()
    }

    pub fn get_event_listeners(&self) -> Arc<Mutex<HashMap<Id, mpsc::Sender<()>>>> {
        self.event_listeners.clone()
    }
}
