use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use anyhow::Context;
use memofs::{DirEntry, Vfs};

use crate::{
    snapshot::{InstanceContext, InstanceMetadata, InstanceSnapshot, InstigatingSource},
    syncback::{hash_instance, FsSnapshot, SyncbackReturn, SyncbackSnapshot},
};

use super::{meta_file::DirectoryMetadata, snapshot_from_vfs};

const EMPTY_DIR_KEEP_NAME: &str = ".gitkeep";

pub fn snapshot_dir(
    context: &InstanceContext,
    vfs: &Vfs,
    path: &Path,
    name: &str,
) -> anyhow::Result<Option<InstanceSnapshot>> {
    let mut snapshot = match snapshot_dir_no_meta(context, vfs, path, name)? {
        Some(snapshot) => snapshot,
        None => return Ok(None),
    };

    DirectoryMetadata::read_and_apply_all(vfs, path, &mut snapshot)?;

    Ok(Some(snapshot))
}

/// Snapshot a directory without applying meta files; useful for if the
/// directory's ClassName will change before metadata should be applied. For
/// example, this can happen if the directory contains an `init.client.lua`
/// file.
pub fn snapshot_dir_no_meta(
    context: &InstanceContext,
    vfs: &Vfs,
    path: &Path,
    name: &str,
) -> anyhow::Result<Option<InstanceSnapshot>> {
    let passes_filter_rules = |child: &DirEntry| {
        context
            .path_ignore_rules
            .iter()
            .all(|rule| rule.passes(child.path()))
    };

    let mut snapshot_children = Vec::new();

    for entry in vfs.read_dir(path)? {
        let entry = entry?;

        if !passes_filter_rules(&entry) {
            continue;
        }

        if let Some(child_snapshot) = snapshot_from_vfs(context, vfs, entry.path())? {
            snapshot_children.push(child_snapshot);
        }
    }

    let normalized_path = vfs.normalize(path)?;
    let relevant_paths = vec![
        normalized_path.clone(),
        // TODO: We shouldn't need to know about Lua existing in this
        // middleware. Should we figure out a way for that function to add
        // relevant paths to this middleware?
        normalized_path.clone().join("init.lua"),
        normalized_path.clone().join("init.luau"),
        normalized_path.clone().join("init.server.lua"),
        normalized_path.clone().join("init.server.luau"),
        normalized_path.clone().join("init.client.lua"),
        normalized_path.clone().join("init.client.luau"),
        normalized_path.clone().join("init.csv"),
    ];

    let snapshot = InstanceSnapshot::new()
        .name(name)
        .class_name("Folder")
        .children(snapshot_children)
        .metadata(
            InstanceMetadata::new()
                .instigating_source(path)
                .relevant_paths(relevant_paths)
                .context(context),
        );

    Ok(Some(snapshot))
}

pub fn syncback_dir<'sync>(
    snapshot: &SyncbackSnapshot<'sync>,
) -> anyhow::Result<SyncbackReturn<'sync>> {
    let new_inst = snapshot.new_inst();

    let mut dir_syncback = syncback_dir_no_meta(snapshot)?;

    let mut meta = DirectoryMetadata::from_syncback_snapshot(snapshot, snapshot.path.clone())?;
    if let Some(meta) = &mut meta {
        if new_inst.class != "Folder" {
            meta.class_name = Some(new_inst.class);
        }

        if !meta.is_empty() {
            dir_syncback.fs_snapshot.add_file(
                snapshot.path.join("init.meta.json"),
                serde_json::to_vec_pretty(&meta)
                    .context("could not serialize new init.meta.json")?,
            );
        }
    }

    let metadata_empty = meta
        .as_ref()
        .map(DirectoryMetadata::is_empty)
        .unwrap_or_default();
    if new_inst.children().is_empty() && metadata_empty {
        dir_syncback
            .fs_snapshot
            .add_file(snapshot.path.join(EMPTY_DIR_KEEP_NAME), Vec::new())
    }

    Ok(dir_syncback)
}

pub fn syncback_dir_no_meta<'sync>(
    snapshot: &SyncbackSnapshot<'sync>,
) -> anyhow::Result<SyncbackReturn<'sync>> {
    let new_inst = snapshot.new_inst();

    let mut children = Vec::new();
    let mut removed_children = Vec::new();

    // We have to enforce unique child names for the file system.
    let mut child_names = HashSet::with_capacity(new_inst.children().len());
    let mut duplicate_set = HashSet::new();
    for child_ref in new_inst.children() {
        let child = snapshot.get_new_instance(*child_ref).unwrap();
        if !child_names.insert(child.name.to_lowercase()) {
            duplicate_set.insert(child.name.as_str());
        }
    }
    if !duplicate_set.is_empty() {
        if duplicate_set.len() <= 25 {
            anyhow::bail!(
                "Instance has children with duplicate name (case may not exactly match):\n {}",
                duplicate_set.into_iter().collect::<Vec<&str>>().join(", ")
            );
        }
        anyhow::bail!("Instance has more than 25 children with duplicate names");
    }

    if let Some(old_inst) = snapshot.old_inst() {
        let mut old_child_map = HashMap::with_capacity(old_inst.children().len());
        for child in old_inst.children() {
            let inst = snapshot.get_old_instance(*child).unwrap();
            old_child_map.insert(inst.name(), inst);
        }

        for new_child_ref in new_inst.children() {
            let new_child = snapshot.get_new_instance(*new_child_ref).unwrap();
            if let Some(old_child) = old_child_map.remove(new_child.name.as_str()) {
                if old_child.metadata().relevant_paths.is_empty() {
                    log::debug!(
                        "Skipping instance {} because it doesn't exist on the disk",
                        old_child.name()
                    );
                    continue;
                } else if matches!(
                    old_child.metadata().instigating_source,
                    Some(InstigatingSource::ProjectNode { .. })
                ) {
                    log::debug!(
                        "Skipping instance {} because it originates in a project file",
                        old_child.name()
                    );
                    continue;
                }
                // This child exists in both doms. Pass it on.
                children.push(snapshot.with_joined_path(*new_child_ref, Some(old_child.id()))?);
            } else {
                // The child only exists in the the new dom
                children.push(snapshot.with_joined_path(*new_child_ref, None)?);
            }
        }
        // Any children that are in the old dom but not the new one are removed.
        removed_children.extend(old_child_map.into_values());
    } else {
        // There is no old instance. Just add every child.
        for new_child_ref in new_inst.children() {
            children.push(snapshot.with_joined_path(*new_child_ref, None)?);
        }
    }
    let mut fs_snapshot = FsSnapshot::new();

    if let Some(old_ref) = snapshot.old {
        let new_hash = hash_instance(snapshot.project(), snapshot.new_tree(), snapshot.new)
            .expect("new Instance should be hashable");
        let old_hash = hash_instance(snapshot.project(), snapshot.old_tree(), old_ref)
            .expect("old Instance should be hashable");

        if old_hash != new_hash {
            fs_snapshot.add_dir(&snapshot.path);
        } else {
            log::debug!(
                "Skipping reserializing directory {} because old and new tree hash the same",
                new_inst.name
            );
        }
    } else {
        fs_snapshot.add_dir(&snapshot.path);
    }

    Ok(SyncbackReturn {
        fs_snapshot,
        children,
        removed_children,
    })
}

#[cfg(test)]
mod test {
    use super::*;

    use memofs::{InMemoryFs, VfsSnapshot};

    #[test]
    fn empty_folder() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot("/foo", VfsSnapshot::empty_dir())
            .unwrap();

        let vfs = Vfs::new(imfs);

        let instance_snapshot =
            snapshot_dir(&InstanceContext::default(), &vfs, Path::new("/foo"), "foo")
                .unwrap()
                .unwrap();

        insta::assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn folder_in_folder() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(
            "/foo",
            VfsSnapshot::dir([("Child", VfsSnapshot::empty_dir())]),
        )
        .unwrap();

        let vfs = Vfs::new(imfs);

        let instance_snapshot =
            snapshot_dir(&InstanceContext::default(), &vfs, Path::new("/foo"), "foo")
                .unwrap()
                .unwrap();

        insta::assert_yaml_snapshot!(instance_snapshot);
    }
}
