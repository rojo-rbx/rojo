use std::{
    borrow::Cow,
    collections::HashMap,
    path::{Path, PathBuf},
    str,
    sync::{Arc, Mutex},
};

use serde_derive::{Serialize, Deserialize};
use log::{info, trace};
use rbx_tree::{RbxTree, RbxId};

use crate::{
    project::Project,
    message_queue::MessageQueue,
    imfs::{Imfs, ImfsItem},
    path_map::PathMap,
    rbx_snapshot::{SnapshotContext, snapshot_project_tree, snapshot_imfs_path},
    snapshot_reconciler::{InstanceChanges, reify_root, reconcile_subtree},
};

const INIT_SCRIPT: &str = "init.lua";
const INIT_SERVER_SCRIPT: &str = "init.server.lua";
const INIT_CLIENT_SCRIPT: &str = "init.client.lua";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetadataPerPath {
    pub instance_id: Option<RbxId>,
    pub instance_name: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetadataPerInstance {
    pub source_path: Option<PathBuf>,
    pub ignore_unknown_instances: bool,
}

pub struct RbxSession {
    tree: RbxTree,

    // TODO(#105): Change metadata_per_path to PathMap<Vec<MetadataPerPath>> for
    // path aliasing.
    metadata_per_path: PathMap<MetadataPerPath>,
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
        let mut metadata_per_path = PathMap::new();
        let mut metadata_per_instance = HashMap::new();

        let tree = {
            let temp_imfs = imfs.lock().unwrap();
            reify_initial_tree(&project, &temp_imfs, &mut metadata_per_path, &mut metadata_per_instance)
        };

        RbxSession {
            tree,
            metadata_per_path,
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
            let mut path_to_snapshot = self.metadata_per_path.descend(root_path, path);

            // If this is a file that might affect its parent if modified, we
            // should snapshot its parent instead.
            match path_to_snapshot.file_name().unwrap().to_str() {
                Some(INIT_SCRIPT) | Some(INIT_SERVER_SCRIPT) | Some(INIT_CLIENT_SCRIPT) => {
                    path_to_snapshot.pop();
                },
                _ => {},
            }

            trace!("Snapshotting path {}", path_to_snapshot.display());

            let path_metadata = self.metadata_per_path.get(&path_to_snapshot).unwrap();
            let instance_id = path_metadata.instance_id
                .expect("Instance did not exist in tree");

            // If this instance is a sync point, pull its name out of our
            // per-path metadata store.
            let instance_name = path_metadata.instance_name.as_ref()
                .map(|value| Cow::Owned(value.to_owned()));

            let mut context = SnapshotContext {
                metadata_per_path: &mut self.metadata_per_path,
            };
            let maybe_snapshot = snapshot_imfs_path(&imfs, &mut context, &path_to_snapshot, instance_name)
                .unwrap_or_else(|_| panic!("Could not generate instance snapshot for path {}", path_to_snapshot.display()));

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
                instance_id,
                &snapshot,
                &mut self.metadata_per_path,
                &mut self.metadata_per_instance,
                &mut changes,
            );
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
                Some(ImfsItem::Directory(_)) | None => {
                    trace!("Updated path was a directory, ignoring.");
                    return;
                },
                Some(ImfsItem::File(_)) => {},
            }
        }

        self.path_created_or_updated(path);
    }

    pub fn path_removed(&mut self, path: &Path) {
        info!("Path removed: {}", path.display());
        self.metadata_per_path.remove(path);
        self.path_created_or_updated(path);
    }

    pub fn path_renamed(&mut self, from_path: &Path, to_path: &Path) {
        info!("Path renamed from {} to {}", from_path.display(), to_path.display());
        self.metadata_per_path.remove(from_path);
        self.path_created_or_updated(from_path);
        self.path_created_or_updated(to_path);
    }

    pub fn get_tree(&self) -> &RbxTree {
        &self.tree
    }

    pub fn get_instance_metadata(&self, id: RbxId) -> Option<&MetadataPerInstance> {
        self.metadata_per_instance.get(&id)
    }

    pub fn debug_get_metadata_per_path(&self) -> &PathMap<MetadataPerPath> {
        &self.metadata_per_path
    }
}

pub fn construct_oneoff_tree(project: &Project, imfs: &Imfs) -> RbxTree {
    let mut metadata_per_path = PathMap::new();
    let mut metadata_per_instance = HashMap::new();
    reify_initial_tree(project, imfs, &mut metadata_per_path, &mut metadata_per_instance)
}

fn reify_initial_tree(
    project: &Project,
    imfs: &Imfs,
    metadata_per_path: &mut PathMap<MetadataPerPath>,
    metadata_per_instance: &mut HashMap<RbxId, MetadataPerInstance>,
) -> RbxTree {
    let mut context = SnapshotContext {
        metadata_per_path,
    };
    let snapshot = snapshot_project_tree(imfs, &mut context, project)
        .expect("Could not snapshot project tree")
        .expect("Project did not produce any instances");

    let mut changes = InstanceChanges::default();
    let tree = reify_root(&snapshot, metadata_per_path, metadata_per_instance, &mut changes);

    tree
}