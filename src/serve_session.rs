use std::{
    collections::HashSet,
    path::Path,
    sync::{Arc, Mutex, MutexGuard},
    time::Instant,
};

use crate::{
    change_processor::ChangeProcessor,
    common_setup,
    message_queue::MessageQueue,
    project::Project,
    session_id::SessionId,
    snapshot::{AppliedPatchSet, RojoTree},
    vfs::{Vfs, VfsFetcher},
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
    /// updated in real-time. This is derived from the session's VFS and will
    /// eventually be mutable to connected clients.
    tree: Arc<Mutex<RojoTree>>,

    /// An in-memory filesystem containing all of the files relevant for this
    /// live session.
    ///
    /// The main use for accessing it from the session is for debugging issues
    /// with Rojo's live-sync protocol.
    vfs: Arc<Mutex<Vfs<F>>>,

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
    _change_processor: ChangeProcessor,
}

/// Methods that need thread-safety bounds on VfsFetcher are limited to this
/// block to prevent needing to spread Send + Sync + 'static into everything
/// that handles ServeSession.
impl<F: VfsFetcher + Send + 'static> ServeSession<F> {
    /// Start a new serve session from the given in-memory filesystem  and start
    /// path.
    ///
    /// The project file is expected to be loaded out-of-band since it's
    /// currently loaded from the filesystem directly instead of through the
    /// in-memory filesystem layer.
    pub fn new<P: AsRef<Path>>(mut vfs: Vfs<F>, start_path: P) -> Self {
        let start_path = start_path.as_ref();

        log::trace!("Starting new ServeSession at path {}", start_path.display(),);

        let start_time = Instant::now();

        let (root_project, tree) = common_setup::start(start_path, &mut vfs);

        let session_id = SessionId::new();
        let message_queue = MessageQueue::new();

        let tree = Arc::new(Mutex::new(tree));
        let message_queue = Arc::new(message_queue);
        let vfs = Arc::new(Mutex::new(vfs));

        log::trace!("Starting ChangeProcessor");
        let change_processor = ChangeProcessor::start(
            Arc::clone(&tree),
            Arc::clone(&message_queue),
            Arc::clone(&vfs),
        );

        Self {
            start_time,
            session_id,
            root_project,
            tree,
            message_queue,
            vfs,
            _change_processor: change_processor,
        }
    }
}

impl<F: VfsFetcher> ServeSession<F> {
    pub fn tree_handle(&self) -> Arc<Mutex<RojoTree>> {
        Arc::clone(&self.tree)
    }

    pub fn tree(&self) -> MutexGuard<'_, RojoTree> {
        self.tree.lock().unwrap()
    }

    pub fn vfs(&self) -> MutexGuard<'_, Vfs<F>> {
        self.vfs.lock().unwrap()
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

    pub fn project_port(&self) -> Option<u16> {
        self.root_project
            .as_ref()
            .and_then(|project| project.serve_port)
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

/// This module is named to trick Insta into naming the resulting snapshots
/// correctly.
///
/// See https://github.com/mitsuhiko/insta/issues/78
#[cfg(test)]
mod serve_session {
    use super::*;

    use std::{path::PathBuf, time::Duration};

    use insta::assert_yaml_snapshot;
    use maplit::hashmap;
    use rojo_insta_ext::RedactionMap;
    use tokio::{runtime::Runtime, timer::Timeout};

    use crate::{
        tree_view::view_tree,
        vfs::{NoopFetcher, TestFetcher, VfsDebug, VfsEvent, VfsSnapshot},
    };

    #[test]
    fn just_folder() {
        let vfs = Vfs::new(NoopFetcher);

        vfs.debug_load_snapshot("/foo", VfsSnapshot::empty_dir());

        let session = ServeSession::new(vfs, "/foo");

        let mut rm = RedactionMap::new();
        assert_yaml_snapshot!(view_tree(&session.tree(), &mut rm));
    }

    #[test]
    fn project_with_folder() {
        let vfs = Vfs::new(NoopFetcher);

        vfs.debug_load_snapshot(
            "/foo",
            VfsSnapshot::dir(hashmap! {
                "default.project.json" => VfsSnapshot::file(r#"
                {
                    "name": "HelloWorld",
                    "tree": {
                        "$path": "src"
                    }
                }
            "#),
                "src" => VfsSnapshot::dir(hashmap! {
                    "hello.txt" => VfsSnapshot::file("Hello, world!"),
                }),
            }),
        );

        let session = ServeSession::new(vfs, "/foo");

        let mut rm = RedactionMap::new();
        assert_yaml_snapshot!(view_tree(&session.tree(), &mut rm));
    }

    #[test]
    fn script_with_meta() {
        let vfs = Vfs::new(NoopFetcher);

        vfs.debug_load_snapshot(
            "/root",
            VfsSnapshot::dir(hashmap! {
                "test.lua" => VfsSnapshot::file("This is a test."),
                "test.meta.json" => VfsSnapshot::file(r#"{ "ignoreUnknownInstances": true }"#),
            }),
        );

        let session = ServeSession::new(vfs, "/root");

        let mut rm = RedactionMap::new();
        assert_yaml_snapshot!(view_tree(&session.tree(), &mut rm));
    }

    #[test]
    fn change_script_meta() {
        let (state, fetcher) = TestFetcher::new();

        state.load_snapshot(
            "/root",
            VfsSnapshot::dir(hashmap! {
                "test.lua" => VfsSnapshot::file("This is a test."),
                "test.meta.json" => VfsSnapshot::file(r#"{ "ignoreUnknownInstances": true }"#),
            }),
        );

        let vfs = Vfs::new(fetcher);
        let session = ServeSession::new(vfs, "/root");

        let mut redactions = RedactionMap::new();
        assert_yaml_snapshot!(
            "change_script_meta_before",
            view_tree(&session.tree(), &mut redactions)
        );

        state.load_snapshot(
            "/root/test.meta.json",
            VfsSnapshot::file(r#"{ "ignoreUnknownInstances": false }"#),
        );

        let receiver = Timeout::new(
            session.message_queue().subscribe_any(),
            Duration::from_millis(200),
        );
        state.raise_event(VfsEvent::Modified(PathBuf::from("/root/test.meta.json")));

        let mut rt = Runtime::new().unwrap();
        let changes = rt.block_on(receiver).unwrap();

        assert_yaml_snapshot!(
            "change_script_meta_patch",
            redactions.redacted_yaml(changes)
        );
        assert_yaml_snapshot!(
            "change_script_meta_after",
            view_tree(&session.tree(), &mut redactions)
        );
    }

    #[test]
    fn change_txt_file() {
        let (state, fetcher) = TestFetcher::new();

        state.load_snapshot("/foo.txt", VfsSnapshot::file("Hello!"));

        let vfs = Vfs::new(fetcher);
        let session = ServeSession::new(vfs, "/foo.txt");

        let mut redactions = RedactionMap::new();
        assert_yaml_snapshot!(
            "change_txt_file_before",
            view_tree(&session.tree(), &mut redactions)
        );

        state.load_snapshot("/foo.txt", VfsSnapshot::file("World!"));

        let receiver = session.message_queue().subscribe_any();

        state.raise_event(VfsEvent::Modified(PathBuf::from("/foo.txt")));

        let receiver = Timeout::new(receiver, Duration::from_millis(200));

        let mut rt = Runtime::new().unwrap();
        let result = rt.block_on(receiver).unwrap();

        assert_yaml_snapshot!("change_txt_file_patch", redactions.redacted_yaml(result));
        assert_yaml_snapshot!(
            "change_txt_file_after",
            view_tree(&session.tree(), &mut redactions)
        );
    }
}
