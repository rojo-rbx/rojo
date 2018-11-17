use std::{
    sync::{Arc, Mutex},
    io,
};

use crate::{
    message_queue::MessageQueue,
    project::{Project, ProjectNode},
    vfs::Vfs,
    session_id::SessionId,
    rbx_session::RbxSession,
    fs_watcher::FsWatcher,
};

pub struct Session {
    pub project: Arc<Project>,
    pub session_id: SessionId,
    pub message_queue: Arc<MessageQueue>,
    pub rbx_session: Arc<Mutex<RbxSession>>,
    fs_watcher: FsWatcher,
}

fn add_sync_points(vfs: &mut Vfs, project_node: &ProjectNode) -> io::Result<()> {
    match project_node {
        ProjectNode::Instance(node) => {
            for child in node.children.values() {
                add_sync_points(vfs, child)?;
            }
        },
        ProjectNode::SyncPoint(node) => {
            vfs.add_root(&node.path)?;
        },
    }

    Ok(())
}

impl Session {
    pub fn new(project: Project) -> io::Result<Session> {
        let mut vfs = Vfs::new();

        add_sync_points(&mut vfs, &project.tree)
            .expect("Could not add sync points when starting new Rojo session");

        let vfs = Arc::new(Mutex::new(vfs));
        let project = Arc::new(project);
        let message_queue = Arc::new(MessageQueue::new());

        let rbx_session = Arc::new(Mutex::new(RbxSession::new(
            Arc::clone(&project),
            Arc::clone(&vfs),
            Arc::clone(&message_queue),
        )));

        let fs_watcher = FsWatcher::start(
            Arc::clone(&vfs),
            Arc::clone(&rbx_session),
        );

        let session_id = SessionId::new();

        Ok(Session {
            project,
            session_id,
            message_queue,
            rbx_session,
            fs_watcher,
        })
    }

    pub fn get_project(&self) -> &Project {
        &self.project
    }
}