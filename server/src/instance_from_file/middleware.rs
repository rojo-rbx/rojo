use std::{
    borrow::Cow,
    collections::HashMap,
    path::{PathBuf, Path},
};

use rbx_dom_weak::{RbxTree, RbxId};

use crate::{
    imfs::{
        FsResult,
        new::{
            Imfs,
            ImfsEntry,
            ImfsFetcher,
            ImfsSnapshot,
            DirectorySnapshot,
        },
    },
    snapshot::InstanceSnapshot,
};

use super::{snapshot_from_imfs, snapshot_from_instance};

pub type SnapshotInstanceResult<'a> = FsResult<Option<InstanceSnapshot<'a>>>;
pub type SnapshotFileResult = Option<(String, ImfsSnapshot)>;

pub trait SnapshotMiddleware {
    fn from_imfs<F: ImfsFetcher>(
        imfs: &mut Imfs<F>,
        entry: &ImfsEntry,
    ) -> SnapshotInstanceResult<'static>;

    fn from_instance(
        tree: &RbxTree,
        id: RbxId,
    ) -> SnapshotFileResult;

    fn change_affects_paths(
        path: &Path
    ) -> Vec<PathBuf> {
        vec![path.to_path_buf()]
    }
}

pub struct SnapshotDir;

impl SnapshotMiddleware for SnapshotDir {
    fn from_imfs<F: ImfsFetcher>(
        imfs: &mut Imfs<F>,
        entry: &ImfsEntry,
    ) -> SnapshotInstanceResult<'static> {
        let children: Vec<ImfsEntry> = entry.children(imfs)?;

        let mut snapshot_children = Vec::new();

        for child in children.into_iter() {
            if let Some(child_snapshot) = snapshot_from_imfs(imfs, &child)? {
                snapshot_children.push(child_snapshot);
            }
        }

        let instance_name = entry.path()
            .file_name().expect("Could not extract file name")
            .to_str().unwrap().to_string();

        Ok(Some(InstanceSnapshot {
            snapshot_id: None,
            name: Cow::Owned(instance_name),
            class_name: Cow::Borrowed("Folder"),
            properties: HashMap::new(),
            children: snapshot_children,
        }))
    }

    fn from_instance(
        tree: &RbxTree,
        id: RbxId,
    ) -> SnapshotFileResult {
        let instance = tree.get_instance(id).unwrap();

        if instance.class_name != "Folder" {
            return None;
        }

        let mut children = HashMap::new();

        for child_id in instance.get_children_ids() {
            if let Some((name, child)) = snapshot_from_instance(tree, *child_id) {
                children.insert(name, child);
            }
        }

        let snapshot = ImfsSnapshot::Directory(DirectorySnapshot {
            children,
        });

        Some((instance.name.clone(), snapshot))
    }
}