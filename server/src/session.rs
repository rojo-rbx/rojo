use std::{
    sync::{Arc, RwLock},
};

use rand;

use ::{
    message_queue::MessageQueue,
    rbx::RbxTree,
    project::Project,
};

pub struct Session {
    project: Project,
    pub session_id: String,
    pub message_queue: Arc<MessageQueue>,
    pub tree: Arc<RwLock<RbxTree>>,
}

impl Session {
    pub fn new(project: Project) -> Session {
        let session_id = rand::random::<u64>().to_string();

        Session {
            session_id,
            project,
            message_queue: Arc::new(MessageQueue::new()),
            tree: Arc::new(RwLock::new(RbxTree::new())),
        }
    }

    pub fn get_project(&self) -> &Project {
        &self.project
    }
}