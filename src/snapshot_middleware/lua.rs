use std::{borrow::Cow, str};

use maplit::hashmap;
use rbx_dom_weak::RbxValue;

use crate::{
    imfs::{FsResultExt, Imfs, ImfsEntry, ImfsFetcher},
    snapshot::{InstanceMetadata, InstanceSnapshot},
};

use super::{
    dir::SnapshotDir,
    meta_file::AdjacentMetadata,
    middleware::{SnapshotInstanceResult, SnapshotMiddleware},
    util::match_trailing,
};

pub struct SnapshotLua;

impl SnapshotMiddleware for SnapshotLua {
    fn from_imfs<F: ImfsFetcher>(
        imfs: &mut Imfs<F>,
        entry: &ImfsEntry,
    ) -> SnapshotInstanceResult<'static> {
        let file_name = entry.path().file_name().unwrap().to_string_lossy();

        // These paths alter their parent instance, so we don't need to turn
        // them into a script instance here.
        match &*file_name {
            "init.lua" | "init.server.lua" | "init.client.lua" => return Ok(None),
            _ => {}
        }

        if entry.is_file() {
            snapshot_lua_file(imfs, entry)
        } else {
            if let Some(snapshot) = snapshot_init(imfs, entry, "init.lua")? {
                // An `init.lua` file turns its parent into a ModuleScript
                Ok(Some(snapshot))
            } else if let Some(snapshot) = snapshot_init(imfs, entry, "init.server.lua")? {
                // An `init.server.lua` file turns its parent into a Script
                Ok(Some(snapshot))
            } else if let Some(snapshot) = snapshot_init(imfs, entry, "init.client.lua")? {
                // An `init.client.lua` file turns its parent into a LocalScript
                Ok(Some(snapshot))
            } else {
                Ok(None)
            }
        }
    }
}

/// Core routine for turning Lua files into snapshots.
fn snapshot_lua_file<F: ImfsFetcher>(
    imfs: &mut Imfs<F>,
    entry: &ImfsEntry,
) -> SnapshotInstanceResult<'static> {
    let file_name = entry.path().file_name().unwrap().to_string_lossy();

    let (class_name, instance_name) = if let Some(name) = match_trailing(&file_name, ".server.lua")
    {
        ("Script", name)
    } else if let Some(name) = match_trailing(&file_name, ".client.lua") {
        ("LocalScript", name)
    } else if let Some(name) = match_trailing(&file_name, ".lua") {
        ("ModuleScript", name)
    } else {
        return Ok(None);
    };

    let contents = entry.contents(imfs)?;
    let contents_str = str::from_utf8(contents)
        // TODO: Turn into error type
        .expect("File content was not valid UTF-8")
        .to_string();

    let properties = hashmap! {
        "Source".to_owned() => RbxValue::String {
            value: contents_str,
        },
    };

    let meta_path = entry
        .path()
        .with_file_name(format!("{}.meta.json", instance_name));

    let metadata = InstanceMetadata {
        instigating_source: Some(entry.path().to_path_buf().into()),
        relevant_paths: vec![entry.path().to_path_buf(), meta_path.clone()],
        ..Default::default()
    };

    let mut snapshot = InstanceSnapshot {
        snapshot_id: None,
        metadata,
        name: Cow::Owned(instance_name.to_owned()),
        class_name: Cow::Borrowed(class_name),
        properties,
        children: Vec::new(),
    };

    if let Some(meta_entry) = imfs.get(meta_path).with_not_found()? {
        let meta_contents = meta_entry.contents(imfs)?;
        let mut metadata = AdjacentMetadata::from_slice(meta_contents);
        metadata.apply_all(&mut snapshot);
    }

    Ok(Some(snapshot))
}

/// Attempts to snapshot an 'init' Lua script contained inside of a folder with
/// the given name.
///
/// Scripts named `init.lua`, `init.server.lua`, or `init.client.lua` usurp
/// their parents, which acts similarly to `__init__.py` from the Python world.
fn snapshot_init<F: ImfsFetcher>(
    imfs: &mut Imfs<F>,
    folder_entry: &ImfsEntry,
    init_name: &str,
) -> SnapshotInstanceResult<'static> {
    let init_path = folder_entry.path().join(init_name);

    if let Some(init_entry) = imfs.get(init_path).with_not_found()? {
        if let Some(dir_snapshot) = SnapshotDir::from_imfs(imfs, folder_entry)? {
            if let Some(mut init_snapshot) = snapshot_lua_file(imfs, &init_entry)? {
                init_snapshot.name = dir_snapshot.name;
                init_snapshot.children = dir_snapshot.children;
                // TODO: Metadata
                // TODO: Validate directory class name is "Folder"

                return Ok(Some(init_snapshot));
            }
        }
    }

    Ok(None)
}

#[cfg(test)]
mod test {
    use super::*;

    use insta::{assert_yaml_snapshot, with_settings};

    use crate::imfs::{ImfsDebug, ImfsSnapshot, NoopFetcher};

    #[test]
    fn module_from_imfs() {
        let mut imfs = Imfs::new(NoopFetcher);
        let file = ImfsSnapshot::file("Hello there!");

        imfs.debug_load_snapshot("/foo.lua", file);

        let entry = imfs.get("/foo.lua").unwrap();
        let instance_snapshot = SnapshotLua::from_imfs(&mut imfs, &entry).unwrap().unwrap();

        assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn server_from_imfs() {
        let mut imfs = Imfs::new(NoopFetcher);
        let file = ImfsSnapshot::file("Hello there!");

        imfs.debug_load_snapshot("/foo.server.lua", file);

        let entry = imfs.get("/foo.server.lua").unwrap();
        let instance_snapshot = SnapshotLua::from_imfs(&mut imfs, &entry).unwrap().unwrap();

        assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn client_from_imfs() {
        let mut imfs = Imfs::new(NoopFetcher);
        let file = ImfsSnapshot::file("Hello there!");

        imfs.debug_load_snapshot("/foo.client.lua", file);

        let entry = imfs.get("/foo.client.lua").unwrap();
        let instance_snapshot = SnapshotLua::from_imfs(&mut imfs, &entry).unwrap().unwrap();

        assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn module_with_meta() {
        let mut imfs = Imfs::new(NoopFetcher);
        let file = ImfsSnapshot::file("Hello there!");
        let meta = ImfsSnapshot::file(
            r#"
            {
                "ignoreUnknownInstances": true
            }
        "#,
        );

        imfs.debug_load_snapshot("/foo.lua", file);
        imfs.debug_load_snapshot("/foo.meta.json", meta);

        let entry = imfs.get("/foo.lua").unwrap();
        let instance_snapshot = SnapshotLua::from_imfs(&mut imfs, &entry).unwrap().unwrap();

        assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn script_with_meta() {
        let mut imfs = Imfs::new(NoopFetcher);
        let file = ImfsSnapshot::file("Hello there!");
        let meta = ImfsSnapshot::file(
            r#"
            {
                "ignoreUnknownInstances": true
            }
        "#,
        );

        imfs.debug_load_snapshot("/foo.server.lua", file);
        imfs.debug_load_snapshot("/foo.meta.json", meta);

        let entry = imfs.get("/foo.server.lua").unwrap();
        let instance_snapshot = SnapshotLua::from_imfs(&mut imfs, &entry).unwrap().unwrap();

        assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn script_disabled() {
        let mut imfs = Imfs::new(NoopFetcher);
        let file = ImfsSnapshot::file("Hello there!");
        let meta = ImfsSnapshot::file(
            r#"
            {
                "properties": {
                    "Disabled": true
                }
            }
            "#,
        );

        imfs.debug_load_snapshot("/bar.server.lua", file);
        imfs.debug_load_snapshot("/bar.meta.json", meta);

        let entry = imfs.get("/bar.server.lua").unwrap();
        let instance_snapshot = SnapshotLua::from_imfs(&mut imfs, &entry).unwrap().unwrap();

        with_settings!({ sort_maps => true }, {
            assert_yaml_snapshot!(instance_snapshot);
        });
    }
}
