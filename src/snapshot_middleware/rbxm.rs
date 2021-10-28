use std::path::Path;

use anyhow::Context;
use memofs::Vfs;

use crate::{
    load_file::load_file,
    plugin_env::PluginEnv,
    snapshot::{InstanceContext, InstanceMetadata, InstanceSnapshot},
};

pub fn snapshot_rbxm(
    context: &InstanceContext,
    vfs: &Vfs,
    plugin_env: &PluginEnv,
    path: &Path,
    name: &str,
) -> anyhow::Result<Option<InstanceSnapshot>> {
    let contents = load_file(vfs, plugin_env, path)?;
    let temp_tree = rbx_binary::from_reader(contents.as_slice())
        .with_context(|| format!("Malformed rbxm file: {}", path.display()))?;

    let root_instance = temp_tree.root();
    let children = root_instance.children();

    if children.len() == 1 {
        let snapshot = InstanceSnapshot::from_tree(&temp_tree, children[0])
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

#[cfg(test)]
mod test {
    use std::sync::Arc;

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

        let mut vfs = Arc::new(Vfs::new(imfs));

        let plugin_env = PluginEnv::new(Arc::clone(&vfs));
        plugin_env.init().unwrap();

        let instance_snapshot = snapshot_rbxm(
            &InstanceContext::default(),
            &mut vfs,
            &plugin_env,
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
