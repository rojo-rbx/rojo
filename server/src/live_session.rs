use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use failure::Fail;

use crate::{
    fs_watcher::FsWatcher,
    imfs::{Imfs, FsError},
    message_queue::MessageQueue,
    project::Project,
    rbx_session::RbxSession,
    rbx_snapshot::SnapshotError,
    session_id::SessionId,
    snapshot_reconciler::InstanceChanges,
    web::{LiveServer, ServiceDependencies},
};

#[derive(Debug, Fail)]
pub enum LiveSessionError {
    #[fail(display = "{}", _0)]
    Fs(#[fail(cause)] FsError),

    #[fail(display = "{}", _0)]
    Snapshot(#[fail(cause)] SnapshotError),
}

impl_from!(LiveSessionError {
    FsError => Fs,
    SnapshotError => Snapshot,
});

/// Contains all of the state for a Rojo live-sync session.
pub struct LiveSession {
    port: u16,
    project: Arc<Project>,
    session_id: SessionId,
    pub message_queue: Arc<MessageQueue<InstanceChanges>>,
    pub rbx_session: Arc<Mutex<RbxSession>>,
    pub imfs: Arc<Mutex<Imfs>>,
    server: LiveServer,
    _fs_watcher: FsWatcher,
}

impl LiveSession {
    pub fn new(project: Arc<Project>, port: u16) -> Result<LiveSession, LiveSessionError> {
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
        )?));

        let fs_watcher = FsWatcher::start(
            Arc::clone(&imfs),
            Some(Arc::clone(&rbx_session)),
        );

        let session_id = SessionId::new();

        let dependencies = ServiceDependencies {
            session_id,
            serve_place_ids: project.serve_place_ids.clone(),
            message_queue: Arc::clone(&message_queue),
            rbx_session: Arc::clone(&rbx_session),
            imfs: Arc::clone(&imfs),
        };

        let server = LiveServer::start(dependencies, port);

        Ok(LiveSession {
            port,
            session_id,
            project,
            message_queue,
            rbx_session,
            imfs,
            server,
            _fs_watcher: fs_watcher,
        })
    }

    /// Restarts the live session using the given project while preserving the
    /// internal session ID.
    pub fn restart_with_new_project(mut self, project: Arc<Project>) -> Result<LiveSession, LiveSessionError> {
        self.server.stop();

        let mut new_session = LiveSession::new(project, self.port)?;
        new_session.session_id = self.session_id;

        Ok(new_session)
    }

    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    pub fn serve_place_ids(&self) -> &Option<HashSet<u64>> {
        &self.project.serve_place_ids
    }
}