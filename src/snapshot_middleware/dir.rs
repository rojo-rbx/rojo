use std::{borrow::Cow, collections::HashMap};

use rbx_dom_weak::{RbxId, RbxTree};

use crate::{
    imfs::new::{DirectorySnapshot, Imfs, ImfsEntry, ImfsFetcher, ImfsSnapshot},
    snapshot::InstanceSnapshot,
};

use super::{
    middleware::{SnapshotFileResult, SnapshotInstanceResult, SnapshotMiddleware},
    snapshot_from_imfs, snapshot_from_instance,
};

pub struct SnapshotDir;

impl SnapshotMiddleware for SnapshotDir {
    fn from_imfs<F: ImfsFetcher>(
        imfs: &mut Imfs<F>,
        entry: &ImfsEntry,
    ) -> SnapshotInstanceResult<'static> {
        if entry.is_file() {
            return Ok(None);
        }

        let children: Vec<ImfsEntry> = entry.children(imfs)?;

        let mut snapshot_children = Vec::new();

        for child in children.into_iter() {
            if let Some(child_snapshot) = snapshot_from_imfs(imfs, &child)? {
                snapshot_children.push(child_snapshot);
            }
        }

        let instance_name = entry
            .path()
            .file_name()
            .expect("Could not extract file name")
            .to_str()
            .unwrap()
            .to_string();

        Ok(Some(InstanceSnapshot {
            snapshot_id: None,
            source: None, // TODO
            name: Cow::Owned(instance_name),
            class_name: Cow::Borrowed("Folder"),
            properties: HashMap::new(),
            children: snapshot_children,
        }))
    }

    fn from_instance(tree: &RbxTree, id: RbxId) -> SnapshotFileResult {
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

        let snapshot = ImfsSnapshot::Directory(DirectorySnapshot { children });

        Some((instance.name.clone(), snapshot))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use maplit::hashmap;

    use crate::imfs::new::NoopFetcher;

    #[test]
    fn empty_folder() {
        let mut imfs = Imfs::new(NoopFetcher);
        let dir = ImfsSnapshot::dir::<String>(HashMap::new());

        imfs.load_from_snapshot("/foo", dir);

        let entry = imfs.get("/foo").unwrap();
        let instance_snapshot = SnapshotDir::from_imfs(&mut imfs, &entry).unwrap().unwrap();

        assert_eq!(instance_snapshot.name, "foo");
        assert_eq!(instance_snapshot.class_name, "Folder");
        assert_eq!(instance_snapshot.properties, HashMap::new());
        assert_eq!(instance_snapshot.children, Vec::new());
    }

    #[test]
    fn folder_in_folder() {
        let mut imfs = Imfs::new(NoopFetcher);
        let dir = ImfsSnapshot::dir(hashmap! {
            "Child" => ImfsSnapshot::dir::<String>(HashMap::new()),
        });

        imfs.load_from_snapshot("/foo", dir);

        let entry = imfs.get("/foo").unwrap();
        let instance_snapshot = SnapshotDir::from_imfs(&mut imfs, &entry).unwrap().unwrap();

        assert_eq!(instance_snapshot.name, "foo");
        assert_eq!(instance_snapshot.class_name, "Folder");
        assert_eq!(instance_snapshot.properties, HashMap::new());
        assert_eq!(instance_snapshot.children.len(), 1);

        let child = &instance_snapshot.children[0];
        assert_eq!(child.name, "Child");
        assert_eq!(child.class_name, "Folder");
        assert_eq!(child.properties, HashMap::new());
        assert_eq!(child.children, Vec::new());
    }
}
