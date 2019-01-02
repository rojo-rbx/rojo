use std::{
    sync::{Arc, Mutex},
    io,
};

use crate::{
    message_queue::MessageQueue,
    project::Project,
    imfs::Imfs,
    session_id::SessionId,
    rbx_session::RbxSession,
    rbx_snapshot::InstanceChanges,
    fs_watcher::FsWatcher,
};

pub struct Session {
    pub project: Arc<Project>,
    pub session_id: SessionId,
    pub message_queue: Arc<MessageQueue<InstanceChanges>>,
    pub rbx_session: Arc<Mutex<RbxSession>>,
    pub imfs: Arc<Mutex<Imfs>>,
    _fs_watcher: FsWatcher,
}

impl Session {
    pub fn new(project: Project) -> io::Result<Session> {
        let imfs = {
            let mut imfs = Imfs::new();
            imfs.add_roots_from_project(&project)?;

            Arc::new(Mutex::new(imfs))
        };
        let project = Arc::new(project);
        let message_queue = Arc::new(MessageQueue::new());

        let rbx_session = Arc::new(Mutex::new(RbxSession::new(
            Arc::clone(&project),
            Arc::clone(&imfs),
            Arc::clone(&message_queue),
        )));

        let fs_watcher = FsWatcher::start(
            Arc::clone(&imfs),
            Arc::clone(&rbx_session),
        );

        let session_id = SessionId::new();

        Ok(Session {
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