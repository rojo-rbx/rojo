use std::{
    collections::HashSet,
    sync::{Arc, Mutex, MutexGuard},
    time::Instant,
};

use crate::{
    change_processor::ChangeProcessor,
    imfs::{Imfs, ImfsFetcher},
    message_queue::MessageQueue,
    project::Project,
    session_id::SessionId,
    snapshot::{AppliedPatchSet, RojoTree},
};

/// Contains all of the state for a Rojo serve session.
///
/// Nothing here is specific to any Rojo interface. Though the primary way to
/// interact with a serve session is Rojo's HTTP right now, there's no reason
/// why Rojo couldn't expose an IPC or channels-based API for embedding in the
/// future. `ServeSession` would be roughly the right interface to expose for
/// those cases.
pub struct ServeSession<F> {
    /// When the serve session was started. Used only for user-facing
    /// diagnostics.
    start_time: Instant,

    /// The root project for the serve session, if there was one defined.
    ///
    /// This will be defined if a folder with a `default.project.json` file was
    /// used for starting the serve session, or if the user specified a full
    /// path to a `.project.json` file.
    ///
    /// If `root_project` is None, values from the project should be treated as
    /// their defaults.
    root_project: Option<Project>,

    /// A randomly generated ID for this serve session. It's used to ensure that
    /// a client doesn't begin connecting to a different server part way through
    /// an operation that needs to be atomic.
    session_id: SessionId,

    /// The tree of Roblox instances associated with this session that will be
    /// updated in real-time. This is derived from the session's IMFS and will
    /// eventually be mutable to connected clients.
    tree: Arc<Mutex<RojoTree>>,

    /// An in-memory filesystem containing all of the files relevant for this
    /// live session.
    ///
    /// The main use for accessing it from the session is for debugging issues
    /// with Rojo's live-sync protocol.
    imfs: Arc<Mutex<Imfs<F>>>,

    /// A queue of changes that have been applied to `tree` that affect clients.
    ///
    /// Clients to the serve session will subscribe to this queue either
    /// directly or through the HTTP API to be notified of mutations that need
    /// to be applied.
    message_queue: Arc<MessageQueue<AppliedPatchSet>>,

    /// The object responsible for listening to changes from the in-memory
    /// filesystem, applying them, updating the Roblox instance tree, and
    /// routing messages through the session's message queue to any connected
    /// clients.
    change_processor: ChangeProcessor,
}

/// Methods that need thread-safety bounds on ImfsFetcher are limited to this
/// block to prevent needing to spread Send + Sync + 'static into everything
/// that handles ServeSession.
impl<F: ImfsFetcher + Send + 'static> ServeSession<F> {
    pub fn new(imfs: Imfs<F>, tree: RojoTree, root_project: Option<Project>) -> Self {
        let start_time = Instant::now();

        let session_id = SessionId::new();
        let message_queue = MessageQueue::new();

        let tree = Arc::new(Mutex::new(tree));
        let message_queue = Arc::new(message_queue);
        let imfs = Arc::new(Mutex::new(imfs));

        let change_processor = ChangeProcessor::start(
            Arc::clone(&tree),
            Arc::clone(&message_queue),
            Arc::clone(&imfs),
        );

        Self {
            start_time,
            session_id,
            root_project,
            tree,
            message_queue,
            imfs,
            change_processor,
        }
    }
}

impl<F: ImfsFetcher> ServeSession<F> {
    pub fn tree(&self) -> MutexGuard<'_, RojoTree> {
        self.tree.lock().unwrap()
    }

    pub fn imfs(&self) -> MutexGuard<'_, Imfs<F>> {
        self.imfs.lock().unwrap()
    }

    pub fn message_queue(&self) -> &MessageQueue<AppliedPatchSet> {
        &self.message_queue
    }

    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    pub fn project_name(&self) -> Option<&str> {
        self.root_project
            .as_ref()
            .map(|project| project.name.as_str())
    }

    pub fn start_time(&self) -> Instant {
        self.start_time
    }

    pub fn serve_place_ids(&self) -> Option<&HashSet<u64>> {
        self.root_project
            .as_ref()
            .and_then(|project| project.serve_place_ids.as_ref())
    }
}
