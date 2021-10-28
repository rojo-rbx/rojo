use std::{
    borrow::Cow,
    collections::HashSet,
    io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
    time::Instant,
};

use crossbeam_channel::Sender;
use memofs::IoResultExt;
use memofs::Vfs;
use thiserror::Error;

use crate::{
    change_processor::ChangeProcessor,
    lua_ast::Expression,
    message_queue::MessageQueue,
    plugin_env::PluginEnv,
    project::{PluginDescription, Project, ProjectError},
    session_id::SessionId,
    snapshot::{
        apply_patch_set, compute_patch_set, AppliedPatchSet, InstanceContext, InstanceSnapshot,
        PatchSet, RojoTree,
    },
    snapshot_middleware::snapshot_from_vfs,
};

// TODO: Centralize this (copied from json middleware)
fn json_to_lua_value(value: serde_json::Value) -> Expression {
    use serde_json::Value;

    match value {
        Value::Null => Expression::Nil,
        Value::Bool(value) => Expression::Bool(value),
        Value::Number(value) => Expression::Number(value.as_f64().unwrap()),
        Value::String(value) => Expression::String(value),
        Value::Array(values) => {
            Expression::Array(values.into_iter().map(json_to_lua_value).collect())
        }
        Value::Object(values) => Expression::table(
            values
                .into_iter()
                .map(|(key, value)| (key.into(), json_to_lua_value(value)))
                .collect(),
        ),
    }
}

/// Contains all of the state for a Rojo serve session. A serve session is used
/// when we need to build a Rojo tree and possibly rebuild it when input files
/// change.
///
/// Nothing here is specific to any Rojo interface. Though the primary way to
/// interact with a serve session is Rojo's HTTP right now, there's no reason
/// why Rojo couldn't expose an IPC or channels-based API for embedding in the
/// future. `ServeSession` would be roughly the right interface to expose for
/// those cases.
pub struct ServeSession {
    /// The object responsible for listening to changes from the in-memory
    /// filesystem, applying them, updating the Roblox instance tree, and
    /// routing messages through the session's message queue to any connected
    /// clients.
    ///
    /// SHOULD BE DROPPED FIRST! ServeSession and ChangeProcessor communicate
    /// with eachother via channels. If ServeSession hangs up those channels
    /// before dropping the ChangeProcessor, its thread will panic with a
    /// RecvError, causing the main thread to panic on drop.
    ///
    /// Allowed to be unused because it has side effects when dropped.
    #[allow(unused)]
    change_processor: ChangeProcessor,

    /// When the serve session was started. Used only for user-facing
    /// diagnostics.
    start_time: Instant,

    /// The root project for the serve session.
    ///
    /// This will be defined if a folder with a `default.project.json` file was
    /// used for starting the serve session, or if the user specified a full
    /// path to a `.project.json` file.
    root_project: Project,

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
    vfs: Arc<Vfs>,

    /// A queue of changes that have been applied to `tree` that affect clients.
    ///
    /// Clients to the serve session will subscribe to this queue either
    /// directly or through the HTTP API to be notified of mutations that need
    /// to be applied.
    message_queue: Arc<MessageQueue<AppliedPatchSet>>,

    /// A channel to send mutation requests on. These will be handled by the
    /// ChangeProcessor and trigger changes in the tree.
    tree_mutation_sender: Sender<PatchSet>,
}

impl ServeSession {
    /// Start a new serve session from the given in-memory filesystem and start
    /// path.
    ///
    /// The project file is expected to be loaded out-of-band since it's
    /// currently loaded from the filesystem directly instead of through the
    /// in-memory filesystem layer.
    pub fn new<P: AsRef<Path>>(vfs: Vfs, start_path: P) -> Result<Self, ServeSessionError> {
        let start_path = start_path.as_ref();
        let start_time = Instant::now();

        log::trace!("Starting new ServeSession at path {}", start_path.display());

        let project_path;
        if Project::is_project_file(start_path) {
            project_path = Cow::Borrowed(start_path);
        } else {
            project_path = Cow::Owned(start_path.join("default.project.json"));
        }

        log::debug!("Loading project file from {}", project_path.display());

        let root_project = match vfs.read(&project_path).with_not_found()? {
            Some(contents) => Project::load_from_slice(&contents, &project_path)?,
            None => {
                return Err(ServeSessionError::NoProjectFound {
                    path: project_path.to_path_buf(),
                });
            }
        };

        let vfs = Arc::new(vfs);

        let plugin_env = PluginEnv::new(Arc::clone(&vfs));
        match plugin_env.init() {
            Ok(_) => (),
            Err(e) => return Err(ServeSessionError::Plugin { source: e }),
        };

        for plugin_description in root_project.plugins.iter() {
            let default_options = "{}".to_string();
            let (plugin_source, plugin_options) = match plugin_description {
                PluginDescription::Source(source) => (source, default_options),
                PluginDescription::SourceWithOptions { source, options } => {
                    (source, json_to_lua_value(options.to_owned()).to_string())
                }
            };

            match plugin_env.load_plugin(&plugin_source, plugin_options) {
                Ok(_) => (),
                Err(e) => return Err(ServeSessionError::Plugin { source: e }),
            };
        }

        let mut tree = RojoTree::new(InstanceSnapshot::new());

        let root_id = tree.get_root_id();

        let instance_context = InstanceContext::default();

        log::trace!("Generating snapshot of instances from VFS");
        let snapshot = snapshot_from_vfs(&instance_context, &vfs, &plugin_env, &start_path)?
            .expect("snapshot did not return an instance");

        log::trace!("Computing initial patch set");
        let patch_set = compute_patch_set(&snapshot, &tree, root_id);

        log::trace!("Applying initial patch set");
        apply_patch_set(&mut tree, patch_set);

        let session_id = SessionId::new();
        let message_queue = MessageQueue::new();

        let tree = Arc::new(Mutex::new(tree));
        let message_queue = Arc::new(message_queue);
        let plugin_env = Arc::new(Mutex::new(plugin_env));

        let (tree_mutation_sender, tree_mutation_receiver) = crossbeam_channel::unbounded();

        log::trace!("Starting ChangeProcessor");
        let change_processor = ChangeProcessor::start(
            Arc::clone(&tree),
            Arc::clone(&vfs),
            Arc::clone(&plugin_env),
            Arc::clone(&message_queue),
            tree_mutation_receiver,
        );

        Ok(Self {
            change_processor,
            start_time,
            session_id,
            root_project,
            tree,
            message_queue,
            tree_mutation_sender,
            vfs,
        })
    }

    pub fn tree_handle(&self) -> Arc<Mutex<RojoTree>> {
        Arc::clone(&self.tree)
    }

    pub fn tree(&self) -> MutexGuard<'_, RojoTree> {
        self.tree.lock().unwrap()
    }

    pub fn tree_mutation_sender(&self) -> Sender<PatchSet> {
        self.tree_mutation_sender.clone()
    }

    #[allow(unused)]
    pub fn vfs(&self) -> &Vfs {
        &self.vfs
    }

    pub fn message_queue(&self) -> &MessageQueue<AppliedPatchSet> {
        &self.message_queue
    }

    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    pub fn project_name(&self) -> &str {
        &self.root_project.name
    }

    pub fn project_port(&self) -> Option<u16> {
        self.root_project.serve_port
    }

    pub fn place_id(&self) -> Option<u64> {
        self.root_project.place_id
    }

    pub fn game_id(&self) -> Option<u64> {
        self.root_project.game_id
    }

    pub fn start_time(&self) -> Instant {
        self.start_time
    }

    pub fn serve_place_ids(&self) -> Option<&HashSet<u64>> {
        self.root_project.serve_place_ids.as_ref()
    }
}

#[derive(Debug, Error)]
pub enum ServeSessionError {
    #[error(
        "Rojo requires a project file, but no project file was found in path {}\n\
        See https://rojo.space/docs/ for guides and documentation.",
        .path.display()
    )]
    NoProjectFound { path: PathBuf },

    #[error(transparent)]
    Io {
        #[from]
        source: io::Error,
    },

    #[error(transparent)]
    Project {
        #[from]
        source: ProjectError,
    },

    #[error(transparent)]
    Other {
        #[from]
        source: anyhow::Error,
    },

    #[error(transparent)]
    Plugin {
        #[from]
        source: rlua::Error,
    },
}
