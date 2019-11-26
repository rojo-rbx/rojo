use std::collections::HashMap;

use rbx_dom_weak::{RbxId, RbxTree};

use crate::{
    snapshot::{InstanceMetadata, InstanceSnapshot},
    vfs::{DirectorySnapshot, FsResultExt, Vfs, VfsEntry, VfsFetcher, VfsSnapshot},
};

use super::{
    context::InstanceSnapshotContext,
    error::SnapshotError,
    meta_file::DirectoryMetadata,
    middleware::{SnapshotFileResult, SnapshotInstanceResult, SnapshotMiddleware},
    snapshot_from_instance, snapshot_from_vfs,
};

pub struct SnapshotDir;

impl SnapshotMiddleware for SnapshotDir {
    fn from_vfs<F: VfsFetcher>(
        context: &InstanceSnapshotContext,
        vfs: &Vfs<F>,
        entry: &VfsEntry,
    ) -> SnapshotInstanceResult {
        if entry.is_file() {
            return Ok(None);
        }

        let children: Vec<VfsEntry> = entry.children(vfs)?;

        let mut snapshot_children = Vec::new();

        for child in children.into_iter() {
            if let Some(child_snapshot) = snapshot_from_vfs(context, vfs, &child)? {
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

        let meta_path = entry.path().join("init.meta.json");

        let mut snapshot = InstanceSnapshot::new()
            .name(instance_name)
            .class_name("Folder")
            .children(snapshot_children)
            .metadata(
                InstanceMetadata::new()
                    .instigating_source(entry.path())
                    .relevant_paths(&[
                        entry.path().to_path_buf(),
                        meta_path.clone(),
                        // TODO: We shouldn't need to know about Lua existing in this
                        // middleware. Should we figure out a way for that function to add
                        // relevant paths to this middleware?
                        entry.path().join("init.lua"),
                        entry.path().join("init.server.lua"),
                        entry.path().join("init.client.lua"),
                    ])
                    .context(context),
            );

        if let Some(meta_entry) = vfs.get(meta_path).with_not_found()? {
            let meta_contents = meta_entry.contents(vfs)?;
            let mut metadata = DirectoryMetadata::from_slice(&meta_contents);
            metadata.apply_all(&mut snapshot);
        }

        Ok(Some(snapshot))
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

        let snapshot = VfsSnapshot::Directory(DirectorySnapshot { children });

        Some((instance.name.clone(), snapshot))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use insta::assert_yaml_snapshot;
    use maplit::hashmap;

    use crate::vfs::{NoopFetcher, VfsDebug};

    #[test]
    fn empty_folder() {
        let mut vfs = Vfs::new(NoopFetcher);
        let dir = VfsSnapshot::dir::<String>(HashMap::new());

        vfs.debug_load_snapshot("/foo", dir);

        let entry = vfs.get("/foo").unwrap();
        let instance_snapshot =
            SnapshotDir::from_vfs(&InstanceSnapshotContext::default(), &mut vfs, &entry)
                .unwrap()
                .unwrap();

        assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn folder_in_folder() {
        let mut vfs = Vfs::new(NoopFetcher);
        let dir = VfsSnapshot::dir(hashmap! {
            "Child" => VfsSnapshot::dir::<String>(HashMap::new()),
        });

        vfs.debug_load_snapshot("/foo", dir);

        let entry = vfs.get("/foo").unwrap();
        let instance_snapshot =
            SnapshotDir::from_vfs(&InstanceSnapshotContext::default(), &mut vfs, &entry)
                .unwrap()
                .unwrap();

        assert_yaml_snapshot!(instance_snapshot);
    }
}
