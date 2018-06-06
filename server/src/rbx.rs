use std::borrow::Cow;
use std::collections::HashMap;

use id::Id;

// TODO: Switch to enum to represent more value types
pub type RbxValue = String;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RbxInstance {
    /// Maps to the `Name` property on Instance.
    pub name: String,

    /// Maps to the `ClassName` property on Instance.
    pub class_name: String,

    /// Contains all other properties of an Instance.
    pub properties: HashMap<String, RbxValue>,

    /// All of the children of this instance. Order is relevant to preserve!
    pub children: Vec<Id>,

    pub parent: Option<Id>,
}

impl<'a> From<&'a RbxInstance> for Cow<'a, RbxInstance> {
    fn from(instance: &'a RbxInstance) -> Cow<'a, RbxInstance> {
        Cow::Borrowed(instance)
    }
}

pub struct RbxTree {
    instances: HashMap<Id, RbxInstance>,
}

impl RbxTree {
    pub fn new() -> RbxTree {
        RbxTree {
            instances: HashMap::new(),
        }
    }

    pub fn get_all_instances(&self) -> &HashMap<Id, RbxInstance> {
        &self.instances
    }

    pub fn insert_instance(&mut self, id: Id, instance: RbxInstance) {
        if let Some(parent_id) = instance.parent {
            if let Some(mut parent) = self.instances.get_mut(&parent_id) {
                if !parent.children.contains(&id) {
                    parent.children.push(id);
                }
            }
        }

        self.instances.insert(id, instance);
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

    pub fn get_instance<'a, 'b, T>(&'a self, id: Id, output: &'b mut HashMap<Id, T>)
        where T: From<&'a RbxInstance>
    {
        let mut ids_to_visit = vec![id];

        loop {
            let id = match ids_to_visit.pop() {
                Some(id) => id,
                None => break,
            };

            match self.instances.get(&id) {
                Some(instance) => {
                    output.insert(id, instance.into());

                    for child_id in &instance.children {
                        ids_to_visit.push(*child_id);
                    }
                },
                None => continue,
            }
        }
    }
}
