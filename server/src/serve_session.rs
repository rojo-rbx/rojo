use std::{
    sync::Arc,
};

use ::{
    message_session::MessageSession,
    rbx::RbxTree,
    project::Project,
};

pub struct ServeSession {
    project: Project,
    messages: Arc<MessageSession>,
    tree: Arc<RbxTree>,
}

impl ServeSession {
    pub fn new(project: Project) -> ServeSession {
        ServeSession {
            project,
            messages: Arc::new(MessageSession::new()),
            tree: Arc::new(RbxTree::new()),
        }
    }

    pub fn get_project(&self) -> &Project {
        &self.project
    }

    pub fn get_messages(&self) -> &MessageSession {
        &self.messages
    }

    pub fn get_tree(&self) -> &RbxTree {
        &self.tree
    }
}