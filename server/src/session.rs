use std::{
    sync::Arc,
};

use ::{
    message_queue::MessageQueue,
    rbx::RbxTree,
    project::Project,
};

pub struct Session {
    project: Project,
    message_queue: Arc<MessageQueue>,
    tree: Arc<RbxTree>,
}

impl Session {
    pub fn new(project: Project) -> Session {
        Session {
            project,
            message_queue: Arc::new(MessageQueue::new()),
            tree: Arc::new(RbxTree::new()),
        }
    }

    pub fn get_project(&self) -> &Project {
        &self.project
    }

    pub fn get_message_queue(&self) -> &MessageQueue {
        &self.message_queue
    }

    pub fn get_tree(&self) -> &RbxTree {
        &self.tree
    }
}