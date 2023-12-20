use std::path::Path;

use anyhow::Context;
use memofs::Vfs;

use crate::{
    snapshot::{InstanceContext, InstanceMetadata, InstanceSnapshot},
    syncback::{FsSnapshot, SyncbackReturn, SyncbackSnapshot},
};

#[profiling::function]
pub fn snapshot_rbxm(
    context: &InstanceContext,
    vfs: &Vfs,
    path: &Path,
    name: &str,
) -> anyhow::Result<Option<InstanceSnapshot>> {
    let temp_tree = rbx_binary::from_reader(vfs.read(path)?.as_slice())
        .with_context(|| format!("Malformed rbxm file: {}", path.display()))?;

    let root_instance = temp_tree.root();
    let children = root_instance.children();

    if children.len() == 1 {
        let child = children[0];
        let snapshot = InstanceSnapshot::from_tree(temp_tree, child)
            .name(name)
            .metadata(
                InstanceMetadata::new()
                    .instigating_source(path)
                    .relevant_paths(vec![path.to_path_buf()])
                    .context(context),
            );

        Ok(Some(snapshot))
    } else {
        anyhow::bail!(
            "Rojo currently only supports model files with one top-level instance.\n\n \
             Check the model file at path {}",
            path.display()
        );
    }
}

pub fn syncback_rbxm<'new, 'old>(
    snapshot: &SyncbackSnapshot<'new, 'old>,
) -> anyhow::Result<SyncbackReturn<'new, 'old>> {
    // If any of the children of this Instance are scripts, we don't want
    // include them in the model. So instead, we'll check and then serialize.
    let inst = snapshot.new_inst();
    let mut path = snapshot.parent_path.join(&snapshot.name);
    path.set_extension("rbxm");
    // Long-term, anyway. Right now we stay silly.
    let mut serialized = Vec::new();
    rbx_binary::to_writer(&mut serialized, snapshot.new_tree(), &[inst.referent()])
        .context("failed to serialize new rbxm")?;

    Ok(SyncbackReturn {
        inst_snapshot: InstanceSnapshot::from_instance(inst),
        fs_snapshot: FsSnapshot::new().with_file(&path, serialized),
        children: Vec::new(),
        removed_children: Vec::new(),
    })
}

#[cfg(test)]
mod test {
    use super::*;

    use memofs::{InMemoryFs, VfsSnapshot};

    #[test]
    fn model_from_vfs() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(
            "/foo.rbxm",
            VfsSnapshot::file(include_bytes!("../../assets/test-folder.rbxm").to_vec()),
        )
        .unwrap();

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_rbxm(
            &InstanceContext::default(),
            &mut vfs,
            Path::new("/foo.rbxm"),
            "foo",
        )
        .unwrap()
        .unwrap();

        assert_eq!(instance_snapshot.name, "foo");
        assert_eq!(instance_snapshot.class_name, "Folder");
        assert_eq!(instance_snapshot.children, Vec::new());

        // We intentionally don't assert on properties. rbx_binary does not
        // distinguish between String and BinaryString. The sample model was
        // created by Roblox Studio and has an empty BinaryString "Tags"
        // property that currently deserializes incorrectly.
        // See: https://github.com/Roblox/rbx-dom/issues/49
    }
}
