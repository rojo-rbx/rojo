use std::{path::Path, str};

use anyhow::Context;
use maplit::hashmap;
use memofs::{IoResultExt, Vfs};

use crate::snapshot::{InstanceContext, InstanceMetadata, InstanceSnapshot};

use super::{
    dir::{dir_meta, snapshot_dir_no_meta},
    meta_file::AdjacentMetadata,
    util::match_trailing,
};

/// Core routine for turning Lua files into snapshots.
pub fn snapshot_lua(
    context: &InstanceContext,
    vfs: &Vfs,
    path: &Path,
) -> anyhow::Result<Option<InstanceSnapshot>> {
    let file_name = path.file_name().unwrap().to_string_lossy();

    let (class_name, instance_name) = if let Some(name) = match_trailing(&file_name, ".server.lua")
    {
        ("Script", name)
    } else if let Some(name) = match_trailing(&file_name, ".client.lua") {
        ("LocalScript", name)
    } else if let Some(name) = match_trailing(&file_name, ".lua") {
        ("ModuleScript", name)
    } else if let Some(name) = match_trailing(&file_name, ".server.luau") {
        ("Script", name)
    } else if let Some(name) = match_trailing(&file_name, ".client.luau") {
        ("LocalScript", name)
    } else if let Some(name) = match_trailing(&file_name, ".luau") {
        ("ModuleScript", name)
    } else {
        return Ok(None);
    };

    let contents = vfs.read(path)?;
    let contents_str = str::from_utf8(&contents)
        .with_context(|| format!("File was not valid UTF-8: {}", path.display()))?
        .to_owned();

    let meta_path = path.with_file_name(format!("{}.meta.json", instance_name));

    let mut snapshot = InstanceSnapshot::new()
        .name(instance_name)
        .class_name(class_name)
        .properties(hashmap! {
            "Source".to_owned() => contents_str.into(),
        })
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

/// Attempts to snapshot an 'init' Lua script contained inside of a folder with
/// the given name.
///
/// Scripts named `init.lua`, `init.server.lua`, or `init.client.lua` usurp
/// their parents, which acts similarly to `__init__.py` from the Python world.
pub fn snapshot_lua_init(
    context: &InstanceContext,
    vfs: &Vfs,
    init_path: &Path,
) -> anyhow::Result<Option<InstanceSnapshot>> {
    let folder_path = init_path.parent().unwrap();
    let dir_snapshot = snapshot_dir_no_meta(context, vfs, folder_path)?.unwrap();

    if dir_snapshot.class_name != "Folder" {
        anyhow::bail!(
            "init.lua, init.server.lua, and init.client.lua can \
             only be used if the instance produced by the containing \
             directory would be a Folder.\n\
             \n\
             The directory {} turned into an instance of class {}.",
            folder_path.display(),
            dir_snapshot.class_name
        );
    }

    let mut init_snapshot = snapshot_lua(context, vfs, init_path)?.unwrap();

    init_snapshot.name = dir_snapshot.name;
    init_snapshot.children = dir_snapshot.children;
    init_snapshot.metadata = dir_snapshot.metadata;

    if let Some(mut meta) = dir_meta(vfs, folder_path)? {
        meta.apply_all(&mut init_snapshot)?;
    }

    Ok(Some(init_snapshot))
}

#[cfg(test)]
mod test {
    use super::*;

    use memofs::{InMemoryFs, VfsSnapshot};

    #[test]
    fn module_from_vfs() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot("/foo.lua", VfsSnapshot::file("Hello there!"))
            .unwrap();

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot =
            snapshot_lua(&InstanceContext::default(), &mut vfs, Path::new("/foo.lua"))
                .unwrap()
                .unwrap();

        insta::assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn server_from_vfs() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot("/foo.server.lua", VfsSnapshot::file("Hello there!"))
            .unwrap();

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_lua(
            &InstanceContext::default(),
            &mut vfs,
            Path::new("/foo.server.lua"),
        )
        .unwrap()
        .unwrap();

        insta::assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn client_from_vfs() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot("/foo.client.lua", VfsSnapshot::file("Hello there!"))
            .unwrap();

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_lua(
            &InstanceContext::default(),
            &mut vfs,
            Path::new("/foo.client.lua"),
        )
        .unwrap()
        .unwrap();

        insta::assert_yaml_snapshot!(instance_snapshot);
    }

    #[ignore = "init.lua functionality has moved to the root snapshot function"]
    #[test]
    fn init_module_from_vfs() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(
            "/root",
            VfsSnapshot::dir(hashmap! {
                "init.lua" => VfsSnapshot::file("Hello!"),
            }),
        )
        .unwrap();

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot =
            snapshot_lua(&InstanceContext::default(), &mut vfs, Path::new("/root"))
                .unwrap()
                .unwrap();

        insta::assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn module_with_meta() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot("/foo.lua", VfsSnapshot::file("Hello there!"))
            .unwrap();
        imfs.load_snapshot(
            "/foo.meta.json",
            VfsSnapshot::file(
                r#"
                    {
                        "ignoreUnknownInstances": true
                    }
                "#,
            ),
        )
        .unwrap();

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot =
            snapshot_lua(&InstanceContext::default(), &mut vfs, Path::new("/foo.lua"))
                .unwrap()
                .unwrap();

        insta::assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn script_with_meta() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot("/foo.server.lua", VfsSnapshot::file("Hello there!"))
            .unwrap();
        imfs.load_snapshot(
            "/foo.meta.json",
            VfsSnapshot::file(
                r#"
                    {
                        "ignoreUnknownInstances": true
                    }
                "#,
            ),
        )
        .unwrap();

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_lua(
            &InstanceContext::default(),
            &mut vfs,
            Path::new("/foo.server.lua"),
        )
        .unwrap()
        .unwrap();

        insta::assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn script_disabled() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot("/bar.server.lua", VfsSnapshot::file("Hello there!"))
            .unwrap();
        imfs.load_snapshot(
            "/bar.meta.json",
            VfsSnapshot::file(
                r#"
                    {
                        "properties": {
                            "Disabled": true
                        }
                    }
                "#,
            ),
        )
        .unwrap();

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_lua(
            &InstanceContext::default(),
            &mut vfs,
            Path::new("/bar.server.lua"),
        )
        .unwrap()
        .unwrap();

        insta::with_settings!({ sort_maps => true }, {
            insta::assert_yaml_snapshot!(instance_snapshot);
        });
    }
}
