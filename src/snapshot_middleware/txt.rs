use std::{path::Path, str};

use anyhow::Context as _;
use memofs::Vfs;
use rbx_dom_weak::types::Variant;
use rbx_dom_weak::ustr;

use crate::{
    snapshot::{InstanceContext, InstanceMetadata, InstanceSnapshot},
    syncback::{FsSnapshot, SyncbackReturn, SyncbackSnapshot},
};

use super::{meta_file::AdjacentMetadata, PathExt as _};

pub fn snapshot_txt(
    context: &InstanceContext,
    vfs: &Vfs,
    path: &Path,
    name: &str,
) -> anyhow::Result<Option<InstanceSnapshot>> {
    let contents = vfs.read_to_string(path)?;
    let contents_str = contents.as_str();

    let mut snapshot = InstanceSnapshot::new()
        .name(name)
        .class_name("StringValue")
        .property(ustr("Value"), contents_str)
        .metadata(
            InstanceMetadata::new()
                .instigating_source(path)
                .relevant_paths(vec![vfs.normalize(path)?])
                .context(context),
        );

    AdjacentMetadata::read_and_apply_all(vfs, path, name, &mut snapshot)?;

    Ok(Some(snapshot))
}

pub fn syncback_txt<'sync>(
    snapshot: &SyncbackSnapshot<'sync>,
) -> anyhow::Result<SyncbackReturn<'sync>> {
    let new_inst = snapshot.new_inst();

    let contents = if let Some(Variant::String(source)) = new_inst.properties.get(&ustr("Value")) {
        source.as_bytes().to_vec()
    } else {
        anyhow::bail!("StringValues must have a `Value` property that is a String");
    };
    let mut fs_snapshot = FsSnapshot::new();
    fs_snapshot.add_file(&snapshot.path, contents);

    let meta = AdjacentMetadata::from_syncback_snapshot(snapshot, snapshot.path.clone())?;
    if let Some(mut meta) = meta {
        // StringValues have relatively few properties that we care about, so
        // shifting is fine.
        meta.properties.shift_remove(&ustr("Value"));

        if !meta.is_empty() {
            let parent = snapshot.path.parent_err()?;
            fs_snapshot.add_file(
                parent.join(format!("{}.meta.json", new_inst.name)),
                serde_json::to_vec_pretty(&meta).context("could not serialize metadata")?,
            );
        }
    }

    Ok(SyncbackReturn {
        fs_snapshot,
        children: Vec::new(),
        removed_children: Vec::new(),
    })
}

#[cfg(test)]
mod test {
    use super::*;

    use memofs::{InMemoryFs, VfsSnapshot};

    #[test]
    fn instance_from_vfs() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot("/foo.txt", VfsSnapshot::file("Hello there!"))
            .unwrap();

        let vfs = Vfs::new(imfs.clone());

        let instance_snapshot = snapshot_txt(
            &InstanceContext::default(),
            &vfs,
            Path::new("/foo.txt"),
            "foo",
        )
        .unwrap()
        .unwrap();

        insta::assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn with_metadata() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot("/foo.txt", VfsSnapshot::file("Hello there!"))
            .unwrap();
        imfs.load_snapshot(
            "/foo.meta.json",
            VfsSnapshot::file(
                r#"{
                    "id": "manually specified"
                }"#,
            ),
        )
        .unwrap();
        let vfs = Vfs::new(imfs.clone());

        let instance_snapshot = snapshot_txt(
            &InstanceContext::default(),
            &vfs,
            Path::new("/foo.txt"),
            "foo",
        )
        .unwrap()
        .unwrap();

        insta::assert_yaml_snapshot!(instance_snapshot);
    }
}
