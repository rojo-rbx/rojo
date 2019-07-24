use std::{
    borrow::Cow,
    collections::HashMap,
    str,
    path::{PathBuf, Path},
};

use rbx_dom_weak::{RbxTree, RbxId, RbxValue };
use maplit::hashmap;

use crate::imfs::new::{Imfs, ImfsEntry, ImfsFetcher};

use super::snapshot::InstanceSnapshot;

pub enum ImfsSnapshot {
    File(FileSnapshot),
    Directory(DirectorySnapshot),
}

pub struct FileSnapshot {
    contents: Vec<u8>,
}

pub struct DirectorySnapshot {
    children: HashMap<String, ImfsSnapshot>,
}

type SnapshotInstanceResult<'a> = Option<InstanceSnapshot<'a>>;
type SnapshotFileResult = Option<(String, ImfsSnapshot)>;

pub trait SnapshotMiddleware {
    fn from_imfs<F: ImfsFetcher>(
        imfs: &mut Imfs<F>,
        entry: ImfsEntry,
    ) -> SnapshotInstanceResult;

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

fn snapshot<F: ImfsFetcher>(imfs: &mut Imfs<F>, entry: &ImfsEntry) -> SnapshotInstanceResult<'static> {
    unimplemented!();
}

fn snapshot_instance(tree: &RbxTree, id: RbxId) -> SnapshotFileResult {
    unimplemented!();
}

pub struct SnapshotDir;

impl SnapshotMiddleware for SnapshotDir {
    fn from_imfs<F: ImfsFetcher>(
        imfs: &mut Imfs<F>,
        entry: ImfsEntry,
    ) -> SnapshotInstanceResult {
        let children = entry.children(imfs)?;

        let mut snapshot_children = Vec::new();

        for child in children.into_iter() {
            if let Some(child_snapshot) = snapshot(imfs, &child) {
                snapshot_children.push(child_snapshot);
            }
        }

        let instance_name = entry.path()
            .file_name().expect("Could not extract file name")
            .to_str().unwrap().to_string();

        Some(InstanceSnapshot {
            snapshot_id: None,
            name: Cow::Owned(instance_name),
            class_name: Cow::Borrowed("Folder"),
            properties: HashMap::new(),
            children: snapshot_children,
        })
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
            if let Some((name, child)) = snapshot_instance(tree, *child_id) {
                children.insert(name, child);
            }
        }

        let snapshot = ImfsSnapshot::Directory(DirectorySnapshot {
            children,
        });

        Some((instance.name.clone(), snapshot))
    }
}

pub struct SnapshotTxt;

impl SnapshotMiddleware for SnapshotTxt {
    fn from_imfs<F: ImfsFetcher>(
        imfs: &mut Imfs<F>,
        entry: ImfsEntry,
    ) -> SnapshotInstanceResult {
        if entry.is_directory() {
            return None;
        }

        let extension = entry.path().extension()?.to_str().unwrap();

        if extension != "txt" {
            return None;
        }

        let instance_name = entry.path()
            .file_stem().expect("Could not extract file stem")
            .to_str().unwrap().to_string();

        let contents = entry.contents(imfs)?;
        let contents_str = str::from_utf8(contents).unwrap().to_string();

        let properties = hashmap! {
            "Value".to_owned() => RbxValue::String {
                value: contents_str,
            },
        };

        Some(InstanceSnapshot {
            snapshot_id: None,
            name: Cow::Owned(instance_name),
            class_name: Cow::Borrowed("StringValue"),
            properties,
            children: Vec::new(),
        })
    }

    fn from_instance(
        tree: &RbxTree,
        id: RbxId,
    ) -> SnapshotFileResult {
        let instance = tree.get_instance(id).unwrap();

        if instance.class_name != "StringValue" {
            return None;
        }

        if !instance.get_children_ids().is_empty() {
            return None;
        }

        let value = match instance.properties.get("Value") {
            Some(RbxValue::String { value }) => value.clone(),
            Some(_) => panic!("wrong type ahh"),
            None => String::new(),
        };

        let snapshot = ImfsSnapshot::File(FileSnapshot {
            contents: value.into_bytes(),
        });

        Some((instance.name.clone(), snapshot))
    }
}