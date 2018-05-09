use std::collections::HashMap;

use id::Id;
use partition::Partition;
use rbx::RbxInstance;
use session::SessionConfig;

pub struct RbxSession {
    pub config: SessionConfig,

    /// The RbxInstance that represents each partition.
    pub partition_instances: HashMap<String, Id>,

    /// The store of all instances in the session.
    pub instances: HashMap<Id, RbxInstance>,

    // pub instances_by_route: HashMap<FileRoute, Id>,
}

impl RbxSession {
    pub fn new(config: SessionConfig) -> RbxSession {
        RbxSession {
            config,
            partition_instances: HashMap::new(),
            instances: HashMap::new(),
        }
    }

    // fn load_instances(&mut self) {
    //     for (partition_name, file_item) in &self.partition_files {
    //         let partition = self.partitions.get(partition_name).unwrap();
    //         let (root_id, mut instances) = file_to_instance(&file_item, partition);

    //         // there has to be an std method for this
    //         // oh well
    //         for (instance_id, instance) in instances.drain() {
    //             self.instances.insert(instance_id, instance);
    //         }

    //         self.partition_instances.insert(partition_name.clone(), root_id);
    //     }
    // }
}
