use std::str;

use maplit::hashmap;
use rbx_dom_weak::RbxValue;

use crate::{
    snapshot::{InstanceContext, InstanceMetadata, InstanceSnapshot},
    vfs::{FsResultExt, Vfs, VfsEntry, VfsFetcher},
};

use super::{
    dir::SnapshotDir,
    meta_file::AdjacentMetadata,
    middleware::{SnapshotInstanceResult, SnapshotMiddleware},
    util::match_trailing,
};

pub struct SnapshotLua;

impl SnapshotMiddleware for SnapshotLua {
    fn from_vfs<F: VfsFetcher>(
        context: &InstanceContext,
        vfs: &Vfs<F>,
        entry: &VfsEntry,
    ) -> SnapshotInstanceResult {
        let file_name = entry.path().file_name().unwrap().to_string_lossy();

        // These paths alter their parent instance, so we don't need to turn
        // them into a script instance here.
        match &*file_name {
            "init.lua" | "init.server.lua" | "init.client.lua" => return Ok(None),
            _ => {}
        }

        if entry.is_file() {
            snapshot_lua_file(context, vfs, entry)
        } else {
            // At this point, our entry is definitely a directory!

            if let Some(snapshot) = snapshot_init(context, vfs, entry, "init.lua")? {
                // An `init.lua` file turns its parent into a ModuleScript
                Ok(Some(snapshot))
            } else if let Some(snapshot) = snapshot_init(context, vfs, entry, "init.server.lua")? {
                // An `init.server.lua` file turns its parent into a Script
                Ok(Some(snapshot))
            } else if let Some(snapshot) = snapshot_init(context, vfs, entry, "init.client.lua")? {
                // An `init.client.lua` file turns its parent into a LocalScript
                Ok(Some(snapshot))
            } else {
                Ok(None)
            }
        }
    }
}

/// Core routine for turning Lua files into snapshots.
fn snapshot_lua_file<F: VfsFetcher>(
    context: &InstanceContext,
    vfs: &Vfs<F>,
    entry: &VfsEntry,
) -> SnapshotInstanceResult {
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

    let contents = entry.contents(vfs)?;
    let contents_str = str::from_utf8(&contents)
        // TODO: Turn into error type
        .expect("File content was not valid UTF-8")
        .to_string();

    let meta_path = entry
        .path()
        .with_file_name(format!("{}.meta.json", instance_name));

    let mut snapshot = InstanceSnapshot::new()
        .name(instance_name)
        .class_name(class_name)
        .properties(hashmap! {
            "Source".to_owned() => RbxValue::String {
                value: contents_str,
            },
        })
        .metadata(
            InstanceMetadata::new()
                .instigating_source(entry.path())
                .relevant_paths(vec![entry.path().to_path_buf(), meta_path.clone()])
                .context(context),
        );

    if let Some(meta_entry) = vfs.get(meta_path).with_not_found()? {
        let meta_contents = meta_entry.contents(vfs)?;
        let mut metadata = AdjacentMetadata::from_slice(&meta_contents);
        metadata.apply_all(&mut snapshot);
    }

    Ok(Some(snapshot))
}

/// Attempts to snapshot an 'init' Lua script contained inside of a folder with
/// the given name.
///
/// Scripts named `init.lua`, `init.server.lua`, or `init.client.lua` usurp
/// their parents, which acts similarly to `__init__.py` from the Python world.
fn snapshot_init<F: VfsFetcher>(
    context: &InstanceContext,
    vfs: &Vfs<F>,
    folder_entry: &VfsEntry,
    init_name: &str,
) -> SnapshotInstanceResult {
    let init_path = folder_entry.path().join(init_name);

    if let Some(init_entry) = vfs.get(init_path).with_not_found()? {
        if let Some(dir_snapshot) = SnapshotDir::from_vfs(context, vfs, folder_entry)? {
            if let Some(mut init_snapshot) = snapshot_lua_file(context, vfs, &init_entry)? {
                if dir_snapshot.class_name != "Folder" {
                    panic!(
                        "init.lua, init.server.lua, and init.client.lua can \
                         only be used if the instance produced by the parent \
                         directory would be a Folder."
                    );
                }

                init_snapshot.name = dir_snapshot.name;
                init_snapshot.children = dir_snapshot.children;
                init_snapshot.metadata = dir_snapshot.metadata;

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

    use crate::vfs::{NoopFetcher, VfsDebug, VfsSnapshot};

    #[test]
    fn module_from_vfs() {
        let mut vfs = Vfs::new(NoopFetcher);
        let file = VfsSnapshot::file("Hello there!");

        vfs.debug_load_snapshot("/foo.lua", file);

        let entry = vfs.get("/foo.lua").unwrap();
        let instance_snapshot =
            SnapshotLua::from_vfs(&InstanceContext::default(), &mut vfs, &entry)
                .unwrap()
                .unwrap();

        assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn server_from_vfs() {
        let mut vfs = Vfs::new(NoopFetcher);
        let file = VfsSnapshot::file("Hello there!");

        vfs.debug_load_snapshot("/foo.server.lua", file);

        let entry = vfs.get("/foo.server.lua").unwrap();
        let instance_snapshot =
            SnapshotLua::from_vfs(&InstanceContext::default(), &mut vfs, &entry)
                .unwrap()
                .unwrap();

        assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn client_from_vfs() {
        let mut vfs = Vfs::new(NoopFetcher);
        let file = VfsSnapshot::file("Hello there!");

        vfs.debug_load_snapshot("/foo.client.lua", file);

        let entry = vfs.get("/foo.client.lua").unwrap();
        let instance_snapshot =
            SnapshotLua::from_vfs(&InstanceContext::default(), &mut vfs, &entry)
                .unwrap()
                .unwrap();

        assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn init_module_from_vfs() {
        let mut vfs = Vfs::new(NoopFetcher);
        let dir = VfsSnapshot::dir(hashmap! {
            "init.lua" => VfsSnapshot::file("Hello!"),
        });

        vfs.debug_load_snapshot("/root", dir);

        let entry = vfs.get("/root").unwrap();
        let instance_snapshot =
            SnapshotLua::from_vfs(&InstanceContext::default(), &mut vfs, &entry)
                .unwrap()
                .unwrap();

        assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn module_with_meta() {
        let mut vfs = Vfs::new(NoopFetcher);
        let file = VfsSnapshot::file("Hello there!");
        let meta = VfsSnapshot::file(
            r#"
            {
                "ignoreUnknownInstances": true
            }
        "#,
        );

        vfs.debug_load_snapshot("/foo.lua", file);
        vfs.debug_load_snapshot("/foo.meta.json", meta);

        let entry = vfs.get("/foo.lua").unwrap();
        let instance_snapshot =
            SnapshotLua::from_vfs(&InstanceContext::default(), &mut vfs, &entry)
                .unwrap()
                .unwrap();

        assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn script_with_meta() {
        let mut vfs = Vfs::new(NoopFetcher);
        let file = VfsSnapshot::file("Hello there!");
        let meta = VfsSnapshot::file(
            r#"
            {
                "ignoreUnknownInstances": true
            }
        "#,
        );

        vfs.debug_load_snapshot("/foo.server.lua", file);
        vfs.debug_load_snapshot("/foo.meta.json", meta);

        let entry = vfs.get("/foo.server.lua").unwrap();
        let instance_snapshot =
            SnapshotLua::from_vfs(&InstanceContext::default(), &mut vfs, &entry)
                .unwrap()
                .unwrap();

        assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn script_disabled() {
        let mut vfs = Vfs::new(NoopFetcher);
        let file = VfsSnapshot::file("Hello there!");
        let meta = VfsSnapshot::file(
            r#"
            {
                "properties": {
                    "Disabled": true
                }
            }
            "#,
        );

        vfs.debug_load_snapshot("/bar.server.lua", file);
        vfs.debug_load_snapshot("/bar.meta.json", meta);

        let entry = vfs.get("/bar.server.lua").unwrap();
        let instance_snapshot =
            SnapshotLua::from_vfs(&InstanceContext::default(), &mut vfs, &entry)
                .unwrap()
                .unwrap();

        with_settings!({ sort_maps => true }, {
            assert_yaml_snapshot!(instance_snapshot);
        });
    }
}
