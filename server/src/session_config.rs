use std::collections::HashMap;

use partition::Partition;

#[derive(Debug, Clone, PartialEq)]
pub struct SessionConfig {
    pub partitions: HashMap<String, Partition>,
}
