use std::collections::HashSet;

use crate::{project::Project, session_id::SessionId, snapshot::RojoTree};

/// Contains all of the state for a Rojo serve session.
pub struct ServeSession {
    root_project: Option<Project>,
    session_id: SessionId,
    tree: RojoTree,
}

impl ServeSession {
    pub fn new(tree: RojoTree, root_project: Option<Project>) -> ServeSession {
        let session_id = SessionId::new();

        ServeSession {
            session_id,
            root_project,
            tree,
        }
    }

    pub fn tree(&self) -> &RojoTree {
        &self.tree
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
