use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use file_route::FileRoute;
use id::{Id, get_id};
use partition::Partition;
use rbx::RbxInstance;
use session::SessionConfig;
use vfs_session::{VfsSession, FileItem};

fn file_to_instances(file_item: &FileItem, partition: &Partition, output: &mut HashMap<Id, RbxInstance>) -> Id {
    match file_item {
        FileItem::File { contents, route } => {
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
        FileItem::Directory { children, route } => {
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

pub struct RbxSession {
    config: SessionConfig,

    vfs_session: Arc<RwLock<VfsSession>>,

    /// The RbxInstance that represents each partition.
    partition_instances: HashMap<String, Id>,

    /// The store of all instances in the session.
    instances: HashMap<Id, RbxInstance>,

    // instances_by_route: HashMap<FileRoute, Id>,
}

impl RbxSession {
    pub fn new(config: SessionConfig, vfs_session: Arc<RwLock<VfsSession>>) -> RbxSession {
        RbxSession {
            config,
            vfs_session,
            partition_instances: HashMap::new(),
            instances: HashMap::new(),
            // instances_by_route: HashMap::new(),
        }
    }

    pub fn read_partitions(&mut self) {
        let vfs_session = self.vfs_session.read().unwrap();

        for partition in self.config.partitions.values() {
            let route = FileRoute {
                partition: partition.name.clone(),
                route: Vec::new(),
            };
            let file_item = vfs_session.get_file_item(&route).unwrap();
            let root_id = file_to_instances(file_item, partition, &mut self.instances);

            self.partition_instances.insert(partition.name.clone(), root_id);
        }
    }
}
