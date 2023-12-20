use std::path::Path;

use anyhow::Context;
use memofs::Vfs;

use crate::{
    snapshot::{InstanceContext, InstanceMetadata, InstanceSnapshot},
    syncback::{FsSnapshot, SyncbackReturn, SyncbackSnapshot},
};

pub fn snapshot_rbxmx(
    context: &InstanceContext,
    vfs: &Vfs,
    path: &Path,
    name: &str,
) -> anyhow::Result<Option<InstanceSnapshot>> {
    let options = rbx_xml::DecodeOptions::new()
        .property_behavior(rbx_xml::DecodePropertyBehavior::ReadUnknown);

    let temp_tree = rbx_xml::from_reader(vfs.read(path)?.as_slice(), options)
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

pub fn syncback_rbxmx<'new, 'old>(
    snapshot: &SyncbackSnapshot<'new, 'old>,
) -> anyhow::Result<SyncbackReturn<'new, 'old>> {
    // If any of the children of this Instance are scripts, we don't want
    // include them in the model. So instead, we'll check and then serialize.

    let inst = snapshot.new_inst();
    let mut path = snapshot.parent_path.join(&snapshot.name);
    path.set_extension("rbxmx");
    // Long-term, anyway. Right now we stay silly.
    let mut serialized = Vec::new();
    rbx_xml::to_writer_default(&mut serialized, snapshot.new_tree(), &[inst.referent()])
        .context("failed to serialize new rbxmx")?;

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
    fn plain_folder() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(
            "/foo.rbxmx",
            VfsSnapshot::file(
                r#"
                    <roblox version="4">
                        <Item class="Folder" referent="0">
                            <Properties>
                                <string name="Name">THIS NAME IS IGNORED</string>
                            </Properties>
                        </Item>
                    </roblox>
                "#,
            ),
        )
        .unwrap();

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_rbxmx(
            &InstanceContext::default(),
            &mut vfs,
            Path::new("/foo.rbxmx"),
            "foo",
        )
        .unwrap()
        .unwrap();

        assert_eq!(instance_snapshot.name, "foo");
        assert_eq!(instance_snapshot.class_name, "Folder");
        assert_eq!(instance_snapshot.properties, Default::default());
        assert_eq!(instance_snapshot.children, Vec::new());
    }
}
