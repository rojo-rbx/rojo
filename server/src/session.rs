use std::collections::HashMap;
use std::sync::{mpsc, Arc, RwLock};
use std::thread;

use id::{Id, get_id};
use file_route::FileRoute;
use partition::Partition;
use rbx::RbxInstance;
use rbx_session::RbxSession;
use vfs_session::{VfsSession, FileItem};
use partition_watcher::PartitionWatcher;

#[derive(Debug, Clone, PartialEq)]
pub struct SessionConfig {
    pub partitions: HashMap<String, Partition>,
}

/// Stub trait for middleware
trait Middleware {
}

fn file_to_instances(file_item: &FileItem, partition: &Partition, output: &mut HashMap<Id, RbxInstance>) -> Id {
    match file_item {
        &FileItem::File { ref contents, ref route } => {
            let mut properties = HashMap::new();
            properties.insert("Value".to_string(), contents.clone());

            let primary_id = get_id();

            output.insert(primary_id, RbxInstance {
                name: route.name(partition).to_string(),
                class_name: "StringValue".to_string(),
                parent: None,
                properties,
            });

            primary_id
        },
        &FileItem::Directory { ref children, ref route } => {
            let primary_id = get_id();

            output.insert(primary_id, RbxInstance {
                name: route.name(partition).to_string(),
                class_name: "Folder".to_string(),
                parent: None,
                properties: HashMap::new(),
            });

            for child_file_item in children.values() {
                let mut child_instances = HashMap::new();
                let child_id = file_to_instances(child_file_item, partition, &mut child_instances);

                child_instances.get_mut(&child_id).unwrap().parent = Some(primary_id);

                for (instance_id, instance) in child_instances.drain() {
                    output.insert(instance_id, instance);
                }
            }

            primary_id
        }
    }
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
        Session {
            vfs_session: Arc::new(RwLock::new(VfsSession::new(config.clone()))),
            rbx_session: Arc::new(RwLock::new(RbxSession::new(config.clone()))),
            middlewares: Vec::new(),
            watchers: Vec::new(),
            config,
        }
    }

    pub fn start(&mut self) {
        let (tx, rx) = mpsc::channel();

        for partition in self.config.partitions.values() {
            let watcher = PartitionWatcher::start_new(partition.clone(), tx.clone());

            self.watchers.push(watcher);
        }

        thread::spawn(move || {
            loop {
                match rx.recv() {
                    Ok((partition_name, change)) => {
                        println!("Got change {:?} on partition {}", change, partition_name);
                    },
                    Err(_) => break,
                }
            }
        });
    }

    pub fn get_vfs_session(&self) -> Arc<RwLock<VfsSession>> {
        self.vfs_session.clone()
    }

    pub fn get_rbx_session(&self) -> Arc<RwLock<RbxSession>> {
        self.rbx_session.clone()
    }
}
