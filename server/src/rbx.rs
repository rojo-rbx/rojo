use std::collections::HashMap;

use id::{Id, get_id};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RbxValue {
    String {
        value: String,
    },
    Number {
        value: f64,
    },
    Bool {
        value: bool,
    },
    Vector3 {
        value: [f64; 3],
    },
    Color3 {
        value: [u8; 3],
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RbxInstance {
    /// Maps to the `Name` property on Instance.
    pub name: String,

    /// Maps to the `ClassName` property on Instance.
    pub class_name: String,

    /// Contains all other properties of an Instance.
    pub properties: HashMap<String, RbxValue>,

    /// The unique ID of the instance
    id: Id,

    /// All of the children of this instance. Order is relevant to preserve!
    children: Vec<Id>,

    /// The parent of the instance, if there is one.
    parent: Option<Id>,
}

impl RbxInstance {
    pub fn get_id(&self) -> Id {
        self.id
    }
}

pub struct Descendants<'a> {
    tree: &'a RbxTree,
    ids_to_visit: Vec<Id>,
}

impl<'a> Iterator for Descendants<'a> {
    type Item = &'a RbxInstance;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let id = match self.ids_to_visit.pop() {
                Some(id) => id,
                None => break,
            };

            match self.tree.get_instance(id) {
                Some(instance) => {
                    for child_id in &instance.children {
                        self.ids_to_visit.push(*child_id);
                    }

                    return Some(instance);
                },
                None => continue,
            }
        }

        None
    }
}

pub struct RbxTree {
    instances: HashMap<Id, RbxInstance>,
    // TODO: Make private
    pub root_instance_id: Id,
}

impl RbxTree {
    pub fn new() -> RbxTree {
        let root_instance_id = get_id();
        let root_instance = RbxInstance {
            name: "game".to_string(),
            class_name: "DataModel".to_string(),
            properties: HashMap::new(),
            id: root_instance_id,
            children: Vec::new(),
            parent: None,
        };

        let mut instances = HashMap::new();
        instances.insert(root_instance_id, root_instance);

        RbxTree {
            instances,
            root_instance_id,
        }
    }

    pub fn get_instance(&self, id: Id) -> Option<&RbxInstance> {
        self.instances.get(&id)
    }

    pub fn get_all_instances(&self) -> &HashMap<Id, RbxInstance> {
        &self.instances
    }

    // TODO: Test this function!
    pub fn insert_tree(&mut self, parent_id: Id, tree: &RbxTree) {
        let mut to_visit = vec![tree.root_instance_id];

        loop {
            let id = match to_visit.pop() {
                Some(id) => id,
                None => break,
            };

            let mut new_child = tree.get_instance(id).unwrap().clone();

            for child in &new_child.children {
                to_visit.push(*child);
            }

            if id == tree.root_instance_id {
                new_child.parent = Some(parent_id);
            }

            self.insert_instance(new_child);
        }
    }

    pub fn insert_instance(&mut self, mut instance: RbxInstance) {
        match instance.parent {
            Some(parent_id) => {
                match self.instances.get_mut(&parent_id) {
                    Some(mut parent) => {
                        if !parent.children.contains(&instance.id) {
                            parent.children.push(instance.id);
                        }
                    },
                    None => {
                        panic!("Tree consistency error, parent {} was not present in tree.", parent_id);
                    }
                }
            },
            None => {
                instance.parent = Some(self.root_instance_id);
            },
        }

        self.instances.insert(instance.id, instance);
    }

    pub fn delete_instance(&mut self, id: Id) -> Vec<Id> {
        let mut ids_to_visit = vec![id];
        let mut ids_deleted = Vec::new();

        // We only need to explicitly remove a child from the first instance we
        // delete, since all others will descend from this instance.
        let parent_id = match self.instances.get(&id) {
            Some(instance) => instance.parent,
            None => None,
        };

        if let Some(parent_id) = parent_id {
            let mut parent = self.instances.get_mut(&parent_id).unwrap();
            let index = parent.children.iter().position(|&v| v == id).unwrap();

            parent.children.remove(index);
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

    pub fn iter_descendants<'a>(&'a self, id: Id) -> Descendants<'a> {
        match self.get_instance(id) {
            Some(instance) => {
                Descendants {
                    tree: self,
                    ids_to_visit: instance.children.clone(),
                }
            },
            None => {
                Descendants {
                    tree: self,
                    ids_to_visit: vec![],
                }
            },
        }
    }
}