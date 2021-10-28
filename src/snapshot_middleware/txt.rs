use std::{path::Path, str};

use anyhow::Context;
use maplit::hashmap;
use memofs::{IoResultExt, Vfs};

use crate::{
    load_file::load_file,
    plugin_env::PluginEnv,
    snapshot::{InstanceContext, InstanceMetadata, InstanceSnapshot},
};

use super::meta_file::AdjacentMetadata;

pub fn snapshot_txt(
    context: &InstanceContext,
    vfs: &Vfs,
    plugin_env: &PluginEnv,
    path: &Path,
    name: &str,
) -> anyhow::Result<Option<InstanceSnapshot>> {
    let contents = load_file(vfs, plugin_env, path)?;
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

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use super::*;

    use memofs::{InMemoryFs, VfsSnapshot};

    #[test]
    fn instance_from_vfs() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot("/foo.txt", VfsSnapshot::file("Hello there!"))
            .unwrap();

        let mut vfs = Arc::new(Vfs::new(imfs));

        let plugin_env = PluginEnv::new(Arc::clone(&vfs));
        plugin_env.init().unwrap();

        let instance_snapshot = snapshot_txt(
            &InstanceContext::default(),
            &mut vfs,
            &plugin_env,
            Path::new("/foo.txt"),
            "foo",
        )
        .unwrap()
        .unwrap();

        insta::assert_yaml_snapshot!(instance_snapshot);
    }
}
