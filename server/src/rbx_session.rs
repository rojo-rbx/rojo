use std::{
    borrow::Cow,
    collections::{HashSet, HashMap},
    path::{Path, PathBuf},
    str,
    sync::{Arc, Mutex},
};

use rlua::Lua;
use serde_derive::{Serialize, Deserialize};
use log::{info, trace};
use rbx_dom_weak::{RbxTree, RbxId};

use crate::{
    project::{Project, ProjectNode},
    message_queue::MessageQueue,
    imfs::{Imfs, ImfsItem},
    path_map::PathMap,
    rbx_snapshot::{
        SnapshotContext,
        SnapshotPluginContext,
        SnapshotPluginEntry,
        snapshot_project_tree,
        snapshot_project_node,
        snapshot_imfs_path,
    },
    snapshot_reconciler::{InstanceChanges, reify_root, reconcile_subtree},
};

const INIT_SCRIPT: &str = "init.lua";
const INIT_SERVER_SCRIPT: &str = "init.server.lua";
const INIT_CLIENT_SCRIPT: &str = "init.client.lua";

/// `source_path` or `project_definition` or both must both be Some.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MetadataPerInstance {
    pub ignore_unknown_instances: bool,

    /// The path on the filesystem that the instance was read from the
    /// filesystem if it came from the filesystem.
    #[serde(serialize_with = "crate::path_serializer::serialize_option")]
    pub source_path: Option<PathBuf>,

    /// Information about the instance that came from the project that defined
    /// it, if that's where it was defined.
    ///
    /// A key-value pair where the key should be the name of the instance and
    /// the value is the ProjectNode from the instance's project.
    pub project_definition: Option<(String, ProjectNode)>,
}

/// Contains all of the state needed to update an `RbxTree` in real time using
/// the in-memory filesystem, as well as messaging to Rojo clients what
/// instances have actually updated at any point.
pub struct RbxSession {
    tree: RbxTree,

    instances_per_path: PathMap<HashSet<RbxId>>,
    metadata_per_instance: HashMap<RbxId, MetadataPerInstance>,
    message_queue: Arc<MessageQueue<InstanceChanges>>,
    imfs: Arc<Mutex<Imfs>>,
}

impl RbxSession {
    pub fn new(
        project: Arc<Project>,
        imfs: Arc<Mutex<Imfs>>,
        message_queue: Arc<MessageQueue<InstanceChanges>>,
    ) -> RbxSession {
        let mut instances_per_path = PathMap::new();
        let mut metadata_per_instance = HashMap::new();

        let plugin_context = if cfg!(feature = "server-plugins") {
            let lua = Lua::new();
            let mut callback_key = None;

            lua.context(|context| {
                let callback = context.load(r#"
                    return function(snapshot)
                        print("got my snapshot:", snapshot)
                        print("name:", snapshot.name, "class name:", snapshot.className)
                    end"#)
                    .set_name("a cool plugin").unwrap()
                    .call::<(), rlua::Function>(()).unwrap();

                callback_key = Some(context.create_registry_value(callback).unwrap());
            });

            let plugins = vec![
                SnapshotPluginEntry {
                    file_name_filter: String::new(),
                    callback: callback_key.unwrap(),
                }
            ];

            Some(SnapshotPluginContext { lua, plugins })
        } else {
            None
        };

        let context = SnapshotContext {
            plugin_context,
        };

        let tree = {
            let temp_imfs = imfs.lock().unwrap();
            reify_initial_tree(&project, &context, &temp_imfs, &mut instances_per_path, &mut metadata_per_instance)
        };

        RbxSession {
            tree,
            instances_per_path,
            metadata_per_instance,
            message_queue,
            imfs,
        }
    }

    fn path_created_or_updated(&mut self, path: &Path) {
        // TODO: Track paths actually updated in each step so we can ignore
        // redundant changes.
        let mut changes = InstanceChanges::default();

        {
            let imfs = self.imfs.lock().unwrap();
            let root_path = imfs.get_root_for_path(path)
                .expect("Path was outside in-memory filesystem roots");

            // Find the closest instance in the tree that currently exists
            let mut path_to_snapshot = self.instances_per_path.descend(root_path, path);

            // If this is a file that might affect its parent if modified, we
            // should snapshot its parent instead.
            match path_to_snapshot.file_name().unwrap().to_str() {
                Some(INIT_SCRIPT) | Some(INIT_SERVER_SCRIPT) | Some(INIT_CLIENT_SCRIPT) => {
                    path_to_snapshot.pop();
                },
                _ => {},
            }

            trace!("Snapshotting path {}", path_to_snapshot.display());

            let instances_at_path = self.instances_per_path.get(&path_to_snapshot)
                .expect("Metadata did not exist for path")
                .clone();

            let context = SnapshotContext {
                plugin_context: None,
            };

            for instance_id in &instances_at_path {
                let instance_metadata = self.metadata_per_instance.get(&instance_id)
                    .expect("Metadata for instance ID did not exist");

                let maybe_snapshot = match &instance_metadata.project_definition {
                    Some((instance_name, project_node)) => {
                        snapshot_project_node(&context, &imfs, &project_node, Cow::Owned(instance_name.clone()))
                            .unwrap_or_else(|_| panic!("Could not generate instance snapshot for path {}", path_to_snapshot.display()))
                    },
                    None => {
                        snapshot_imfs_path(&context, &imfs, &path_to_snapshot, None)
                            .unwrap_or_else(|_| panic!("Could not generate instance snapshot for path {}", path_to_snapshot.display()))
                    },
                };

                let snapshot = match maybe_snapshot {
                    Some(snapshot) => snapshot,
                    None => {
                        trace!("Path resulted in no snapshot being generated.");
                        return;
                    },
                };

                trace!("Snapshot: {:#?}", snapshot);

                reconcile_subtree(
                    &mut self.tree,
                    *instance_id,
                    &snapshot,
                    &mut self.instances_per_path,
                    &mut self.metadata_per_instance,
                    &mut changes,
                );
            }
        }

        if changes.is_empty() {
            trace!("No instance changes triggered from file update.");
        } else {
            trace!("Pushing changes: {}", changes);
            self.message_queue.push_messages(&[changes]);
        }
    }

    pub fn path_created(&mut self, path: &Path) {
        info!("Path created: {}", path.display());
        self.path_created_or_updated(path);
    }

    pub fn path_updated(&mut self, path: &Path) {
        info!("Path updated: {}", path.display());

        {
            let imfs = self.imfs.lock().unwrap();

            // If the path doesn't exist or is a directory, we don't care if it
            // updated
            match imfs.get(path) {
                Some(ImfsItem::Directory(_)) => {
                    trace!("Updated path was a directory, ignoring.");
                    return;
                },
                None => {
                    trace!("Updated path did not exist in IMFS, ignoring.");
                    return;
                },
                Some(ImfsItem::File(_)) => {},
            }
        }

        self.path_created_or_updated(path);
    }

    pub fn path_removed(&mut self, path: &Path) {
        info!("Path removed: {}", path.display());
        self.instances_per_path.remove(path);
        self.path_created_or_updated(path);
    }

    pub fn path_renamed(&mut self, from_path: &Path, to_path: &Path) {
        info!("Path renamed from {} to {}", from_path.display(), to_path.display());
        self.instances_per_path.remove(from_path);
        self.path_created_or_updated(from_path);
        self.path_created_or_updated(to_path);
    }

    pub fn get_tree(&self) -> &RbxTree {
        &self.tree
    }

    pub fn get_instance_metadata(&self, id: RbxId) -> Option<&MetadataPerInstance> {
        self.metadata_per_instance.get(&id)
    }
}

pub fn construct_oneoff_tree(project: &Project, imfs: &Imfs) -> RbxTree {
    let mut instances_per_path = PathMap::new();
    let mut metadata_per_instance = HashMap::new();
    let context = SnapshotContext {
        plugin_context: None,
    };
    reify_initial_tree(project, &context, imfs, &mut instances_per_path, &mut metadata_per_instance)
}

fn reify_initial_tree(
    project: &Project,
    context: &SnapshotContext,
    imfs: &Imfs,
    instances_per_path: &mut PathMap<HashSet<RbxId>>,
    metadata_per_instance: &mut HashMap<RbxId, MetadataPerInstance>,
) -> RbxTree {
    let snapshot = snapshot_project_tree(&context, imfs, project)
        .expect("Could not snapshot project tree")
        .expect("Project did not produce any instances");

    let mut changes = InstanceChanges::default();
    let tree = reify_root(&snapshot, instances_per_path, metadata_per_instance, &mut changes);

    tree
}