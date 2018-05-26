use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use file_route::FileRoute;
use id::{Id, get_id};
use partition::Partition;
use rbx::RbxInstance;
use session::SessionConfig;
use vfs_session::{VfsSession, FileItem, FileChange};
use message_session::{Message, MessageSession};

fn file_to_instances(
    file_item: &FileItem,
    partition: &Partition,
    output: &mut HashMap<Id, RbxInstance>,
    instances_by_route: &mut HashMap<FileRoute, Id>,
) -> (Id, Vec<Id>) {
    match file_item {
        FileItem::File { contents, route } => {
            let mut properties = HashMap::new();
            properties.insert("Value".to_string(), contents.clone());

            let primary_id = match instances_by_route.get(&file_item.get_route()) {
                Some(&id) => id,
                None => {
                    let id = get_id();
                    instances_by_route.insert(route.clone(), id);

                    id
                },
            };

            let parent = match file_item.get_route().parent() {
                Some(parent_route) => match instances_by_route.get(&parent_route) {
                    Some(parent_id) => Some(*parent_id),
                    None => None,
                },
                None => None,
            };

            output.insert(primary_id, RbxInstance {
                name: route.name(partition).to_string(),
                class_name: "StringValue".to_string(),
                parent,
                properties,
            });

            (primary_id, vec![primary_id])
        },
        FileItem::Directory { children, route } => {
            let primary_id = match instances_by_route.get(&file_item.get_route()) {
                Some(&id) => id,
                None => {
                    let id = get_id();
                    instances_by_route.insert(route.clone(), id);

                    id
                },
            };

            let parent = match file_item.get_route().parent() {
                Some(parent_route) => match instances_by_route.get(&parent_route) {
                    Some(parent_id) => Some(*parent_id),
                    None => None,
                },
                None => None,
            };

            output.insert(primary_id, RbxInstance {
                name: route.name(partition).to_string(),
                class_name: "Folder".to_string(),
                parent,
                properties: HashMap::new(),
            });

            let mut changed_ids = vec![primary_id];

            for child_file_item in children.values() {
                let (child_id, mut child_changed_ids) = file_to_instances(child_file_item, partition, output, instances_by_route);

                output.get_mut(&child_id).unwrap().parent = Some(primary_id);

                changed_ids.push(child_id);
                for id in child_changed_ids.drain(..) {
                    changed_ids.push(id);
                }
            }

            (primary_id, changed_ids)
        },
    }
}

pub struct RbxSession {
    config: SessionConfig,

    vfs_session: Arc<RwLock<VfsSession>>,

    message_session: MessageSession,

    /// The RbxInstance that represents each partition.
    partition_instances: HashMap<String, Id>,

    /// The store of all instances in the session.
    pub instances: HashMap<Id, RbxInstance>,

    /// A map from files in the VFS to instances loaded in the session.
    instances_by_route: HashMap<FileRoute, Id>,
}

impl RbxSession {
    pub fn new(config: SessionConfig, vfs_session: Arc<RwLock<VfsSession>>, message_session: MessageSession) -> RbxSession {
        RbxSession {
            config,
            vfs_session,
            message_session,
            partition_instances: HashMap::new(),
            instances: HashMap::new(),
            instances_by_route: HashMap::new(),
        }
    }

    pub fn read_partitions(&mut self) {
        let vfs_session_arc = self.vfs_session.clone();
        let vfs_session = vfs_session_arc.read().unwrap();

        for partition in self.config.partitions.values() {
            let route = FileRoute {
                partition: partition.name.clone(),
                route: Vec::new(),
            };
            let file_item = vfs_session.get_by_route(&route).unwrap();
            let (root_id, _) = file_to_instances(file_item, partition, &mut self.instances, &mut self.instances_by_route);
            self.partition_instances.insert(partition.name.clone(), root_id);
        }
    }

    pub fn handle_change(&mut self, change: &FileChange) {
        let vfs_session_arc = self.vfs_session.clone();
        let vfs_session = vfs_session_arc.read().unwrap();

        match change {
            FileChange::Created(route) | FileChange::Updated(route) => {
                let file_item = vfs_session.get_by_route(route).unwrap();
                let partition = self.config.partitions.get(&route.partition).unwrap();
                let (_, changed_ids) = file_to_instances(file_item, partition, &mut self.instances, &mut self.instances_by_route);

                let messages = changed_ids
                    .iter()
                    .map(|&id| Message::InstanceChanged { id })
                    .collect::<Vec<_>>();

                self.message_session.push_messages(&messages);
            },
            FileChange::Deleted(route) => {
                match self.instances_by_route.get(route) {
                    Some(id) => {
                        self.instances.remove(id);
                        self.message_session.push_messages(&[Message::InstanceChanged { id: *id }]);
                    },
                    None => (),
                }
            },
            FileChange::Moved(from_route, to_route) => {
            },
        }
    }
}
