use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use file_route::FileRoute;
use id::{Id, get_id};
use partition::Partition;
use rbx::RbxInstance;
use session::SessionConfig;
use vfs_session::{VfsSession, FileItem, FileChange};
use message_session::{Message, MessageSession};

fn insert_if_missing<T: PartialEq>(vec: &mut Vec<T>, value: T) {
    if !vec.contains(&value) {
        vec.push(value);
    }
}

// TODO: Rethink data structure and insertion/update behavior. Maybe break some
// pieces off into a new object?
fn file_to_instances(
    file_item: &FileItem,
    partition: &Partition,
    output: &mut HashMap<Id, RbxInstance>,
    instances_by_route: &mut HashMap<FileRoute, Id>,
    parent_id: Option<Id>,
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

            output.insert(primary_id, RbxInstance {
                name: route.name(partition).to_string(),
                class_name: "StringValue".to_string(),
                properties,
                children: Vec::new(),
                parent: parent_id,
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

            let mut child_ids = Vec::new();

            let mut changed_ids = vec![primary_id];

            for child_file_item in children.values() {
                let (child_id, mut child_changed_ids) = file_to_instances(child_file_item, partition, output, instances_by_route, Some(primary_id));

                child_ids.push(child_id);
                changed_ids.push(child_id);

                // TODO: Should I stop using drain on Vecs of Copyable types?
                for id in child_changed_ids.drain(..) {
                    changed_ids.push(id);
                }
            }

            output.insert(primary_id, RbxInstance {
                name: route.name(partition).to_string(),
                class_name: "Folder".to_string(),
                properties: HashMap::new(),
                children: child_ids,
                parent: parent_id,
            });

            (primary_id, changed_ids)
        },
    }
}

pub struct RbxSession {
    config: SessionConfig,

    vfs_session: Arc<RwLock<VfsSession>>,

    message_session: MessageSession,

    /// The RbxInstance that represents each partition.
    // TODO: Can this be removed in favor of instances_by_route?
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

    pub fn delete_instance(&mut self, id: Id) -> Vec<Id> {
        let mut ids_to_visit = vec![id];
        let mut ids_deleted = Vec::new();

        for instance in self.instances.values_mut() {
            match instance.children.iter().position(|&v| v == id) {
                Some(index) => {
                    instance.children.remove(index);
                },
                None => {},
            }
        }

        loop {
            let id = match ids_to_visit.pop() {
                Some(id) => id,
                None => break,
            };

            match self.instances.get(&id) {
                Some(instance) => ids_to_visit.extend_from_slice(&instance.children),
                None => continue,
            }

            self.instances.remove(&id);
            ids_deleted.push(id);
        }

        ids_deleted
    }

    pub fn get_instance<'a, 'b>(&'a self, id: Id, output: &'b mut HashMap<Id, &'a RbxInstance>) {
        let mut ids_to_visit = vec![id];

        loop {
            let id = match ids_to_visit.pop() {
                Some(id) => id,
                None => break,
            };

            match self.instances.get(&id) {
                Some(instance) => {
                    output.insert(id, instance);

                    for child_id in &instance.children {
                        ids_to_visit.push(*child_id);
                    }
                },
                None => continue,
            }
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

            let parent_id = match route.parent() {
                Some(parent_route) => match self.instances_by_route.get(&parent_route) {
                    Some(&parent_id) => Some(parent_id),
                    None => None,
                },
                None => None,
            };

            let (root_id, _) = file_to_instances(file_item, partition, &mut self.instances, &mut self.instances_by_route, parent_id);

            if let Some(parent_id) = parent_id {
                insert_if_missing(&mut self.instances.get_mut(&parent_id).unwrap().children, root_id);
            }

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

                let parent_id = match route.parent() {
                    Some(parent_route) => match self.instances_by_route.get(&parent_route) {
                        Some(&parent_id) => Some(parent_id),
                        None => None,
                    },
                    None => None,
                };

                let (root_id, changed_ids) = file_to_instances(file_item, partition, &mut self.instances, &mut self.instances_by_route, parent_id);

                if let Some(parent_id) = parent_id {
                    insert_if_missing(&mut self.instances.get_mut(&parent_id).unwrap().children, root_id);
                }

                let messages = changed_ids
                    .iter()
                    .map(|&id| Message::InstanceChanged { id })
                    .collect::<Vec<_>>();

                self.message_session.push_messages(&messages);
            },
            FileChange::Deleted(route) => {
                match self.instances_by_route.get(route) {
                    Some(&id) => {
                        self.delete_instance(id);
                        self.instances_by_route.remove(route);
                        self.message_session.push_messages(&[Message::InstanceChanged { id }]);
                    },
                    None => (),
                }
            },
            FileChange::Moved(from_route, to_route) => {
                let mut messages = Vec::new();

                match self.instances_by_route.get(from_route) {
                    Some(&id) => {
                        self.delete_instance(id);
                        self.instances_by_route.remove(from_route);
                        messages.push(Message::InstanceChanged { id });
                    },
                    None => (),
                }

                let file_item = vfs_session.get_by_route(to_route).unwrap();
                let partition = self.config.partitions.get(&to_route.partition).unwrap();

                let parent_id = match to_route.parent() {
                    Some(parent_route) => match self.instances_by_route.get(&parent_route) {
                        Some(&parent_id) => Some(parent_id),
                        None => None,
                    },
                    None => None,
                };

                let (root_id, changed_ids) = file_to_instances(file_item, partition, &mut self.instances, &mut self.instances_by_route, parent_id);

                if let Some(parent_id) = parent_id {
                    insert_if_missing(&mut self.instances.get_mut(&parent_id).unwrap().children, root_id);
                }

                for id in changed_ids {
                    messages.push(Message::InstanceChanged { id });
                }

                self.message_session.push_messages(&messages);
            },
        }
    }
}
