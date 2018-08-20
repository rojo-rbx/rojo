use std::{
    sync::{Arc, RwLock},
};

use ::{
    message_queue::MessageQueue,
    rbx::RbxTree,
    project::Project,
};

pub struct Session {
    project: Project,
    pub message_queue: Arc<MessageQueue>,
    pub tree: Arc<RwLock<RbxTree>>,
}

impl Session {
    pub fn new(project: Project) -> Session {
        Session {
            project,
            message_queue: Arc::new(MessageQueue::new()),
            tree: Arc::new(RwLock::new(RbxTree::new())),
        }
    }

    pub fn get_project(&self) -> &Project {
        &self.project
    }
}