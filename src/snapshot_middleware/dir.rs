use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::Path,
};

use anyhow::Context;
use memofs::{DirEntry, IoResultExt, Vfs};
use rbx_dom_weak::types::{Ref, Variant};

use crate::{
    resolution::UnresolvedValue,
    snapshot::{InstanceContext, InstanceMetadata, InstanceSnapshot},
    syncback::{FsSnapshot, SyncbackReturn, SyncbackSnapshot},
};

use super::{meta_file::DirectoryMetadata, snapshot_from_vfs};

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

pub fn syncback_dir<'new, 'old>(
    snapshot: &SyncbackSnapshot<'new, 'old>,
) -> anyhow::Result<SyncbackReturn<'new, 'old>> {
    let path = snapshot.parent_path.join(&snapshot.name);
    let new_inst = snapshot.new_inst();

    let mut dir_syncback = syncback_dir_no_meta(snapshot)?;

    let mut meta = if let Some(dir) = dir_meta(snapshot.vfs(), &path)? {
        dir
    } else {
        DirectoryMetadata {
            ignore_unknown_instances: None,
            properties: BTreeMap::new(),
            attributes: BTreeMap::new(),
            class_name: if new_inst.class == "Folder" {
                None
            } else {
                Some(new_inst.class.clone())
            },
            path: path.join("init.meta.json"),
        }
    };
    for (name, value) in snapshot.get_filtered_properties() {
        if name == "Attributes" || name == "AttributesSerialize" {
            if let Variant::Attributes(attrs) = value {
                meta.attributes.extend(attrs.iter().map(|(name, value)| {
                    (
                        name.to_string(),
                        UnresolvedValue::FullyQualified(value.clone()),
                    )
                }))
            } else {
                log::error!("Property {name} should be Attributes but is not");
            }
        } else {
            meta.properties.insert(
                name.to_string(),
                UnresolvedValue::from_variant(value.to_owned(), &new_inst.class, name),
            );
        }
    }

    if !meta.is_empty() {
        dir_syncback.fs_snapshot.push_file(
            &meta.path,
            serde_json::to_vec_pretty(&meta).context("could not serialize new init.meta.json")?,
        );
    }
    Ok(dir_syncback)
}

pub fn syncback_dir_no_meta<'new, 'old>(
    snapshot: &SyncbackSnapshot<'new, 'old>,
) -> anyhow::Result<SyncbackReturn<'new, 'old>> {
    let path = snapshot.parent_path.join(&snapshot.name);

    let new_inst = snapshot.new_inst();

    let mut removed_children = Vec::new();
    let mut children = Vec::new();

    if let Some(old_inst) = snapshot.old_inst() {
        let old_children: HashMap<&str, Ref> = old_inst
            .children()
            .iter()
            .map(|old_ref| {
                (
                    snapshot.get_old_instance(*old_ref).unwrap().name(),
                    *old_ref,
                )
            })
            .collect();
        let new_children: HashSet<&str> = snapshot
            .new_inst()
            .children()
            .iter()
            .map(|new_ref| snapshot.get_new_instance(*new_ref).unwrap().name.as_str())
            .collect();

        for child_ref in old_inst.children() {
            let old_child = snapshot.get_old_instance(*child_ref).unwrap();
            // If it exists in the old tree but not the new one, it was removed.
            if !new_children.contains(old_child.name()) {
                removed_children.push(old_child);
            }
        }

        for child_ref in new_inst.children() {
            let new_child = snapshot.get_new_instance(*child_ref).unwrap();
            // If it exists in the new tree but not the old one, it was added.
            match old_children.get(new_child.name.as_str()) {
                None => {
                    children.push(snapshot.from_parent(new_child.name.clone(), *child_ref, None))
                }
                Some(old_ref) => children.push(snapshot.from_parent(
                    new_child.name.clone(),
                    *child_ref,
                    Some(*old_ref),
                )),
            }
        }
    } else {
        for child_ref in new_inst.children() {
            let child = snapshot.get_new_instance(*child_ref).unwrap();
            children.push(snapshot.from_parent(child.name.clone(), *child_ref, None))
        }
    }

    Ok(SyncbackReturn {
        inst_snapshot: InstanceSnapshot::from_instance(new_inst),
        fs_snapshot: FsSnapshot::new().with_dir(path),
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
