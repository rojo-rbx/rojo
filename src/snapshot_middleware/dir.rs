use std::path::Path;

use memofs::{DirEntry, IoResultExt, Vfs};

use crate::snapshot::{InstanceContext, InstanceMetadata, InstanceSnapshot};
use super::{meta_file::DirectoryMetadata, snapshot_from_vfs};

pub fn snapshot_dir(
    context: &InstanceContext,
    vfs: &Vfs,
    path: &Path,
    child_path: Option<&Path>,
    children: Option<Vec<InstanceSnapshot>>
) -> anyhow::Result<Option<InstanceSnapshot>> {
    let mut snapshot = match snapshot_dir_no_meta(context, vfs, path, child_path,children)? {
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
    child_path: Option<&Path>,
    children: Option<Vec<InstanceSnapshot>>
) -> anyhow::Result<Option<InstanceSnapshot>> {
    let passes_filter_rules = |child: &DirEntry| {
        context
            .path_ignore_rules
            .iter()
            .all(|rule| rule.passes(child.path()))
    };

    let mut snapshot_children = Vec::new();
    match child_path {
        Some(child_path) => {
            println!("childpath {:?}",child_path);
            if let Some(child_snapshot) = snapshot_from_vfs(context, vfs, child_path, None, None)? {
                snapshot_children.push(child_snapshot);
            }
            if let Some(vec_children) = children {
                for child in vec_children {
                    if child_path.ends_with("meta.json") {
                        let meta_file_name = child_path.file_name();
                        let mut child_name = child.name.to_string();
                        child_name.push_str("meta.json");
                        
                        if child.name.to_string() == meta_file_name.unwrap().to_str().unwrap() {
                            if let Some(child_snapshot) = snapshot_from_vfs(context, vfs, child.metadata.relevant_paths.first().unwrap(), None, None)? {
                                snapshot_children.push(child_snapshot);
                            }
                        }else{
                            snapshot_children.push(child.clone())
                        }
                    }else{
                        snapshot_children.push(child.clone())
                    }
                }
            }
        }
        None => {
            println!("childpath None");
            println!("path {:?}", path);
            for entry in vfs.read_dir(path)? {
                let entry = entry?;

                if !passes_filter_rules(&entry) {
                    continue;
                }

                if let Some(child_snapshot) = snapshot_from_vfs(context, vfs, entry.path(), None, None)? {
                    snapshot_children.push(child_snapshot);
                }
            }
        }
    }
    let instance_name = path
        .file_name()
        .expect("Could not extract file name")
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("File name was not valid UTF-8: {}", path.display()))?
        .to_string();

    let meta_path = path.join("init.meta.json");

    let relevant_paths = vec![
        path.to_path_buf(),
        meta_path.clone(),
        // TODO: We shouldn't need to know about Lua existing in this
        // middleware. Should we figure out a way for that function to add
        // relevant paths to this middleware?
        path.join("init.lua"),
        path.join("init.luau"),
        path.join("init.server.lua"),
        path.join("init.server.luau"),
        path.join("init.client.lua"),
        path.join("init.client.luau"),
        path.join("init.csv"),
    ];

    let snapshot = InstanceSnapshot::new()
        .name(instance_name)
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

        let instance_snapshot =
            snapshot_dir(&InstanceContext::default(), &mut vfs, Path::new("/foo"))
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

        let instance_snapshot =
            snapshot_dir(&InstanceContext::default(), &mut vfs, Path::new("/foo"))
                .unwrap()
                .unwrap();

        insta::assert_yaml_snapshot!(instance_snapshot);
    }
}
