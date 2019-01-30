use std::{
    sync::{Arc, Mutex},
};

use crate::{
    fs_watcher::FsWatcher,
    imfs::{Imfs, FsError},
    message_queue::MessageQueue,
    project::Project,
    rbx_session::RbxSession,
    session_id::SessionId,
    snapshot_reconciler::InstanceChanges,
};

/// Contains all of the state for a Rojo live-sync session.
pub struct LiveSession {
    pub project: Arc<Project>,
    pub session_id: SessionId,
    pub message_queue: Arc<MessageQueue<InstanceChanges>>,
    pub rbx_session: Arc<Mutex<RbxSession>>,
    pub imfs: Arc<Mutex<Imfs>>,
    _fs_watcher: FsWatcher,
}

impl LiveSession {
    pub fn new(project: Arc<Project>) -> Result<LiveSession, FsError> {
        let imfs = {
            let mut imfs = Imfs::new();
            imfs.add_roots_from_project(&project)?;

            Arc::new(Mutex::new(imfs))
        };
        let message_queue = Arc::new(MessageQueue::new());

        let rbx_session = Arc::new(Mutex::new(RbxSession::new(
            Arc::clone(&project),
            Arc::clone(&imfs),
            Arc::clone(&message_queue),
        )));

        let fs_watcher = FsWatcher::start(
            Arc::clone(&imfs),
            Some(Arc::clone(&rbx_session)),
        );

        let session_id = SessionId::new();

        Ok(LiveSession {
            project,
            session_id,
            message_queue,
            rbx_session,
            imfs,
            _fs_watcher: fs_watcher,
        })
    }

    pub fn get_project(&self) -> &Project {
        &self.project
    }
}