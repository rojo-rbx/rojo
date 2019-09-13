use std::{
    collections::HashSet,
    sync::{Mutex, MutexGuard},
};

use crate::{
    imfs::new::{Imfs, ImfsFetcher},
    message_queue::MessageQueue,
    project::Project,
    session_id::SessionId,
    snapshot::RojoTree,
};

/// Contains all of the state for a Rojo serve session.
pub struct ServeSession<F> {
    root_project: Option<Project>,
    session_id: SessionId,
    tree: Mutex<RojoTree>,
    message_queue: MessageQueue<()>, // TODO: Real message type
    imfs: Imfs<F>,
}

impl<F: ImfsFetcher> ServeSession<F> {
    pub fn new(imfs: Imfs<F>, tree: RojoTree, root_project: Option<Project>) -> Self {
        let session_id = SessionId::new();
        let message_queue = MessageQueue::new();

        ServeSession {
            session_id,
            root_project,
            tree: Mutex::new(tree),
            message_queue,
            imfs,
        }
    }

    pub fn tree(&self) -> MutexGuard<'_, RojoTree> {
        self.tree.lock().unwrap()
    }

    pub fn imfs(&self) -> &Imfs<F> {
        &self.imfs
    }

    pub fn message_queue(&self) -> &MessageQueue<()> {
        &self.message_queue
    }

    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    pub fn serve_place_ids(&self) -> Option<&HashSet<u64>> {
        self.root_project
            .as_ref()
            .and_then(|project| project.serve_place_ids.as_ref())
    }
}
