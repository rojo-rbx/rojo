use std::{
    borrow::Cow,
    collections::HashMap,
    path::{Path, PathBuf},
    str,
    sync::{Arc, Mutex},
};

use log::{info, trace};
use rbx_tree::{RbxTree, RbxId};

use crate::{
    project::{Project, InstanceProjectNodeMetadata},
    message_queue::MessageQueue,
    imfs::{Imfs, ImfsItem},
    path_map::PathMap,
    rbx_snapshot::{SnapshotMetadata, snapshot_project_tree, snapshot_imfs_path},
    snapshot_reconciler::{InstanceChanges, reify_root, reconcile_subtree},
};

const INIT_SCRIPT: &str = "init.lua";
const INIT_SERVER_SCRIPT: &str = "init.server.lua";
const INIT_CLIENT_SCRIPT: &str = "init.client.lua";

pub struct RbxSession {
    tree: RbxTree,
    path_map: PathMap<RbxId>,
    instance_metadata_map: HashMap<RbxId, InstanceProjectNodeMetadata>,
    sync_point_names: HashMap<PathBuf, String>,
    message_queue: Arc<MessageQueue<InstanceChanges>>,
    imfs: Arc<Mutex<Imfs>>,
}

impl RbxSession {
    pub fn new(
        project: Arc<Project>,
        imfs: Arc<Mutex<Imfs>>,
        message_queue: Arc<MessageQueue<InstanceChanges>>,
    ) -> RbxSession {
        let mut sync_point_names = HashMap::new();
        let mut path_map = PathMap::new();
        let mut instance_metadata_map = HashMap::new();

        let tree = {
            let temp_imfs = imfs.lock().unwrap();
            reify_initial_tree(&project, &temp_imfs, &mut path_map, &mut instance_metadata_map, &mut sync_point_names)
        };

        RbxSession {
            tree,
            path_map,
            instance_metadata_map,
            sync_point_names,
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
            let mut path_to_snapshot = self.path_map.descend(root_path, path);
            let &instance_id = self.path_map.get(&path_to_snapshot).unwrap();

            // If this is a file that might affect its parent if modified, we
            // should snapshot its parent instead.
            match path_to_snapshot.file_name().unwrap().to_str() {
                Some(INIT_SCRIPT) | Some(INIT_SERVER_SCRIPT) | Some(INIT_CLIENT_SCRIPT) => {
                    path_to_snapshot.pop();
                },
                _ => {},
            }

            trace!("Snapshotting path {}", path_to_snapshot.display());

            let instance_name = self.sync_point_names.get(&path_to_snapshot)
                .map(|value| Cow::Owned(value.to_owned()));
            let mut snapshot_meta = SnapshotMetadata {
                sync_point_names: &mut self.sync_point_names,
            };
            let maybe_snapshot = snapshot_imfs_path(&imfs, &mut snapshot_meta, &path_to_snapshot, instance_name)
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
                &mut self.path_map,
                &mut self.instance_metadata_map,
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
        self.path_map.remove(path);
        self.path_created_or_updated(path);
    }

    pub fn path_renamed(&mut self, from_path: &Path, to_path: &Path) {
        info!("Path renamed from {} to {}", from_path.display(), to_path.display());
        self.path_map.remove(from_path);
        self.path_created_or_updated(from_path);
        self.path_created_or_updated(to_path);
    }

    pub fn get_tree(&self) -> &RbxTree {
        &self.tree
    }

    pub fn get_instance_metadata(&self, id: RbxId) -> Option<&InstanceProjectNodeMetadata> {
        self.instance_metadata_map.get(&id)
    }

    pub fn debug_get_path_map(&self) -> &PathMap<RbxId> {
        &self.path_map
    }
}

pub fn construct_oneoff_tree(project: &Project, imfs: &Imfs) -> RbxTree {
    let mut path_map = PathMap::new();
    let mut instance_metadata_map = HashMap::new();
    let mut sync_point_names = HashMap::new();
    reify_initial_tree(project, imfs, &mut path_map, &mut instance_metadata_map, &mut sync_point_names)
}

fn reify_initial_tree(
    project: &Project,
    imfs: &Imfs,
    path_map: &mut PathMap<RbxId>,
    instance_metadata_map: &mut HashMap<RbxId, InstanceProjectNodeMetadata>,
    sync_point_names: &mut HashMap<PathBuf, String>,
) -> RbxTree {
    let mut meta = SnapshotMetadata {
        sync_point_names,
    };
    let snapshot = snapshot_project_tree(imfs, &mut meta, project)
        .expect("Could not snapshot project tree")
        .expect("Project did not produce any instances");

    let mut changes = InstanceChanges::default();
    let tree = reify_root(&snapshot, path_map, instance_metadata_map, &mut changes);

    tree
}