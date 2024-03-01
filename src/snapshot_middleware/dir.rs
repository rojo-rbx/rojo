use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use anyhow::Context;
use memofs::{DirEntry, IoResultExt, Vfs};

use crate::{
    snapshot::{InstanceContext, InstanceMetadata, InstanceSnapshot},
    syncback::{FsSnapshot, SyncbackReturn, SyncbackSnapshot},
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

    if let Some(mut meta) = dir_meta(vfs, path)? {
        meta.apply_all(&mut snapshot)?;
    }

    Ok(Some(snapshot))
}

/// Retrieves the meta file that should be applied for this directory, if it
/// exists.
pub fn dir_meta(vfs: &Vfs, path: &Path) -> anyhow::Result<Option<DirectoryMetadata>> {
    let meta_path = path.join("init.meta.json");

    if let Some(meta_contents) = vfs.read(&meta_path).with_not_found()? {
        let metadata = DirectoryMetadata::from_slice(&meta_contents, meta_path)?;
        Ok(Some(metadata))
    } else {
        Ok(None)
    }
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

    let meta_path = path.join("init.meta.json");

    let relevant_paths = vec![path.to_path_buf(), meta_path];

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
    dir_name: &str,
) -> anyhow::Result<SyncbackReturn<'sync>> {
    let path = snapshot.parent_path.join(dir_name);
    let new_inst = snapshot.new_inst();

    let mut dir_syncback = syncback_dir_no_meta(snapshot, dir_name)?;

    let mut meta = DirectoryMetadata::from_syncback_snapshot(snapshot, path.clone())?;
    if let Some(meta) = &mut meta {
        if new_inst.class != "Folder" {
            meta.class_name = Some(new_inst.class.clone());
        }

        if !meta.is_empty() {
            dir_syncback.fs_snapshot.add_file(
                path.join("init.meta.json"),
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
            .add_file(path.join(EMPTY_DIR_KEEP_NAME), Vec::new())
    }

    Ok(dir_syncback)
}

pub fn syncback_dir_no_meta<'sync>(
    snapshot: &SyncbackSnapshot<'sync>,
    dir_name: &str,
) -> anyhow::Result<SyncbackReturn<'sync>> {
    let path = snapshot.parent_path.join(dir_name);
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
                }
                // This child exists in both doms. Pass it on.
                children.push(snapshot.with_parent(
                    new_child.name.clone(),
                    *new_child_ref,
                    Some(old_child.id()),
                ));
            } else {
                // The child only exists in the the new dom
                children.push(snapshot.with_parent(new_child.name.clone(), *new_child_ref, None));
            }
        }
        // Any children that are in the old dom but not the new one are removed.
        removed_children.extend(old_child_map.into_values());
    } else {
        // There is no old instance. Just add every child.
        for new_child_ref in new_inst.children() {
            let new_child = snapshot.get_new_instance(*new_child_ref).unwrap();
            children.push(snapshot.with_parent(new_child.name.clone(), *new_child_ref, None));
        }
    }
    let mut fs_snapshot = FsSnapshot::new();
    fs_snapshot.add_dir(&path);

    Ok(SyncbackReturn {
        inst_snapshot: InstanceSnapshot::from_instance(new_inst),
        fs_snapshot,
        children,
        removed_children,
    })
}

#[cfg(test)]
mod test {
    use super::*;

    use maplit::hashmap;
    use memofs::{InMemoryFs, VfsSnapshot};

    #[test]
    fn empty_folder() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot("/foo", VfsSnapshot::empty_dir())
            .unwrap();

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_dir(
            &InstanceContext::default(),
            &mut vfs,
            Path::new("/foo"),
            "foo",
        )
        .unwrap()
        .unwrap();

        insta::assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn folder_in_folder() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(
            "/foo",
            VfsSnapshot::dir(hashmap! {
                "Child" => VfsSnapshot::empty_dir(),
            }),
        )
        .unwrap();

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_dir(
            &InstanceContext::default(),
            &mut vfs,
            Path::new("/foo"),
            "foo",
        )
        .unwrap()
        .unwrap();

        insta::assert_yaml_snapshot!(instance_snapshot);
    }
}
