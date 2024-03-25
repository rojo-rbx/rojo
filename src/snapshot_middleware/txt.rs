use std::{path::Path, str};

use anyhow::Context;
use maplit::hashmap;
use memofs::{IoResultExt, Vfs};
use rbx_dom_weak::types::Variant;

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
    let contents = vfs.read(path)?;
    let contents_str = str::from_utf8(&contents)
        .with_context(|| format!("File was not valid UTF-8: {}", path.display()))?
        .to_owned();

    let properties = hashmap! {
        "Value".to_owned() => contents_str.into(),
    };

    let meta_path = path.with_file_name(format!("{}.meta.json", name));

    let mut snapshot = InstanceSnapshot::new()
        .name(name)
        .class_name("StringValue")
        .properties(properties)
        .metadata(
            InstanceMetadata::new()
                .instigating_source(path)
                .relevant_paths(vec![path.to_path_buf(), meta_path.clone()])
                .context(context),
        );

    if let Some(meta_contents) = vfs.read(&meta_path).with_not_found()? {
        let mut metadata = AdjacentMetadata::from_slice(&meta_contents, meta_path)?;
        metadata.apply_all(&mut snapshot)?;
    }

    Ok(Some(snapshot))
}

pub fn syncback_txt<'sync>(
    snapshot: &SyncbackSnapshot<'sync>,
) -> anyhow::Result<SyncbackReturn<'sync>> {
    let new_inst = snapshot.new_inst();

    let contents = if let Some(Variant::String(source)) = new_inst.properties.get("Value") {
        source.as_bytes().to_vec()
    } else {
        anyhow::bail!("StringValues must have a `Value` property that is a String");
    };
    let mut fs_snapshot = FsSnapshot::new();
    fs_snapshot.add_file(&snapshot.path, contents);

    let meta = AdjacentMetadata::from_syncback_snapshot(snapshot, snapshot.path.clone())?;
    if let Some(mut meta) = meta {
        meta.properties.remove("Value");

        if !meta.is_empty() {
            let parent = snapshot.path.parent_err()?;
            fs_snapshot.add_file(
                parent.join(format!("{}.meta.json", new_inst.name)),
                serde_json::to_vec_pretty(&meta).context("could not serialize metadata")?,
            );
        }
    }

    Ok(SyncbackReturn {
        inst_snapshot: InstanceSnapshot::from_instance(new_inst),
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

        let mut vfs = Vfs::new(imfs.clone());

        let instance_snapshot = snapshot_txt(
            &InstanceContext::default(),
            &mut vfs,
            Path::new("/foo.txt"),
            "foo",
        )
        .unwrap()
        .unwrap();

        insta::assert_yaml_snapshot!(instance_snapshot);
    }
}
