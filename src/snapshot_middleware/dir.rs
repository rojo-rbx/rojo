use std::{borrow::Cow, collections::HashMap};

use rbx_dom_weak::{RbxId, RbxTree};

use crate::{
    imfs::{DirectorySnapshot, Imfs, ImfsEntry, ImfsFetcher, ImfsSnapshot},
    snapshot::{InstanceMetadata, InstanceSnapshot},
};

use super::{
    context::InstanceSnapshotContext,
    error::SnapshotError,
    middleware::{SnapshotFileResult, SnapshotInstanceResult, SnapshotMiddleware},
    snapshot_from_imfs, snapshot_from_instance,
};

pub struct SnapshotDir;

impl SnapshotMiddleware for SnapshotDir {
    fn from_imfs<F: ImfsFetcher>(
        context: &mut InstanceSnapshotContext,
        imfs: &mut Imfs<F>,
        entry: &ImfsEntry,
    ) -> SnapshotInstanceResult<'static> {
        if entry.is_file() {
            return Ok(None);
        }

        let children: Vec<ImfsEntry> = entry.children(imfs)?;

        let mut snapshot_children = Vec::new();

        for child in children.into_iter() {
            if let Some(child_snapshot) = snapshot_from_imfs(context, imfs, &child)? {
                snapshot_children.push(child_snapshot);
            }
        }

        let instance_name = entry
            .path()
            .file_name()
            .expect("Could not extract file name")
            .to_str()
            .ok_or_else(|| SnapshotError::file_name_bad_unicode(entry.path()))?
            .to_string();

        Ok(Some(InstanceSnapshot {
            snapshot_id: None,
            metadata: InstanceMetadata {
                instigating_source: Some(entry.path().to_path_buf().into()),
                relevant_paths: vec![entry.path().to_path_buf()],
                ..Default::default()
            },
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

    use insta::assert_yaml_snapshot;
    use maplit::hashmap;

    use crate::imfs::{ImfsDebug, NoopFetcher};

    #[test]
    fn empty_folder() {
        let mut imfs = Imfs::new(NoopFetcher);
        let dir = ImfsSnapshot::dir::<String>(HashMap::new());

        imfs.debug_load_snapshot("/foo", dir);

        let entry = imfs.get("/foo").unwrap();
        let instance_snapshot =
            SnapshotDir::from_imfs(&mut InstanceSnapshotContext::default(), &mut imfs, &entry)
                .unwrap()
                .unwrap();

        assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn folder_in_folder() {
        let mut imfs = Imfs::new(NoopFetcher);
        let dir = ImfsSnapshot::dir(hashmap! {
            "Child" => ImfsSnapshot::dir::<String>(HashMap::new()),
        });

        imfs.debug_load_snapshot("/foo", dir);

        let entry = imfs.get("/foo").unwrap();
        let instance_snapshot =
            SnapshotDir::from_imfs(&mut InstanceSnapshotContext::default(), &mut imfs, &entry)
                .unwrap()
                .unwrap();

        assert_yaml_snapshot!(instance_snapshot);
    }
}
