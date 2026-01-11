use std::{path::Path, str};

use anyhow::Context as _;
use memofs::Vfs;
use rbx_dom_weak::{
    types::{Enum, Variant},
    ustr, HashMapExt as _, UstrMap,
};

use crate::{
    snapshot::{InstanceContext, InstanceMetadata, InstanceSnapshot},
    syncback::{FsSnapshot, SyncbackReturn, SyncbackSnapshot},
};

use super::{
    dir::{snapshot_dir_no_meta, syncback_dir_no_meta},
    meta_file::{AdjacentMetadata, DirectoryMetadata},
    PathExt as _,
};

#[derive(Debug)]
pub enum ScriptType {
    Server,
    Client,
    Module,
    Plugin,
    LegacyServer,
    LegacyClient,
    RunContextServer,
    RunContextClient,
}

/// Core routine for turning Lua files into snapshots.
pub fn snapshot_lua(
    context: &InstanceContext,
    vfs: &Vfs,
    path: &Path,
    name: &str,
    script_type: ScriptType,
) -> anyhow::Result<Option<InstanceSnapshot>> {
    let run_context_enums = &rbx_reflection_database::get()
        .unwrap()
        .enums
        .get("RunContext")
        .expect("Unable to get RunContext enums!")
        .items;

    let (class_name, run_context) = match script_type {
        ScriptType::Server => {
            if context.emit_legacy_scripts {
                ("Script", run_context_enums.get("Legacy"))
            } else {
                ("Script", run_context_enums.get("Server"))
            }
        }
        ScriptType::Client => {
            if context.emit_legacy_scripts {
                ("LocalScript", None)
            } else {
                ("Script", run_context_enums.get("Client"))
            }
        }
        ScriptType::Module => ("ModuleScript", None),
        ScriptType::Plugin => ("Script", run_context_enums.get("Plugin")),
        ScriptType::LegacyServer => ("Script", run_context_enums.get("Legacy")),
        ScriptType::LegacyClient => ("LocalScript", None),
        ScriptType::RunContextServer => ("Script", run_context_enums.get("Server")),
        ScriptType::RunContextClient => ("Script", run_context_enums.get("Client")),
    };

    let contents = vfs.read_to_string_lf_normalized(path)?;
    let contents_str = contents.as_str();

    let mut properties = UstrMap::with_capacity(2);
    properties.insert(ustr("Source"), contents_str.into());

    if let Some(run_context) = run_context {
        properties.insert(
            ustr("RunContext"),
            Enum::from_u32(run_context.to_owned()).into(),
        );
    }

    let mut snapshot = InstanceSnapshot::new()
        .name(name)
        .class_name(class_name)
        .properties(properties)
        .metadata(
            InstanceMetadata::new()
                .instigating_source(path)
                .relevant_paths(vec![vfs.normalize(path)?])
                .context(context),
        );

    AdjacentMetadata::read_and_apply_all(vfs, path, name, &mut snapshot)?;

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
    name: &str,
    script_type: ScriptType,
) -> anyhow::Result<Option<InstanceSnapshot>> {
    let folder_path = init_path.parent().unwrap();
    let dir_snapshot = snapshot_dir_no_meta(context, vfs, folder_path, name)?.unwrap();

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

    let mut init_snapshot =
        snapshot_lua(context, vfs, init_path, &dir_snapshot.name, script_type)?.unwrap();

    init_snapshot.children = dir_snapshot.children;
    init_snapshot.metadata = dir_snapshot.metadata;
    // The directory snapshot middleware includes all possible init paths
    // so we don't need to add it here.

    DirectoryMetadata::read_and_apply_all(vfs, folder_path, &mut init_snapshot)?;

    Ok(Some(init_snapshot))
}

pub fn syncback_lua<'sync>(
    snapshot: &SyncbackSnapshot<'sync>,
) -> anyhow::Result<SyncbackReturn<'sync>> {
    let new_inst = snapshot.new_inst();

    let contents = if let Some(Variant::String(source)) = new_inst.properties.get(&ustr("Source")) {
        source.as_bytes().to_vec()
    } else {
        anyhow::bail!("Scripts must have a `Source` property that is a String")
    };
    let mut fs_snapshot = FsSnapshot::new();
    fs_snapshot.add_file(&snapshot.path, contents);

    let meta = AdjacentMetadata::from_syncback_snapshot(snapshot, snapshot.path.clone())?;
    if let Some(mut meta) = meta {
        // Scripts have relatively few properties that we care about, so shifting
        // is fine.
        meta.properties.shift_remove(&ustr("Source"));

        if !meta.is_empty() {
            let parent_location = snapshot.path.parent_err()?;
            fs_snapshot.add_file(
                parent_location.join(format!("{}.meta.json", new_inst.name)),
                serde_json::to_vec_pretty(&meta).context("cannot serialize metadata")?,
            );
        }
    }

    Ok(SyncbackReturn {
        fs_snapshot,
        // Scripts don't have a child!
        children: Vec::new(),
        removed_children: Vec::new(),
    })
}

pub fn syncback_lua_init<'sync>(
    script_type: ScriptType,
    snapshot: &SyncbackSnapshot<'sync>,
) -> anyhow::Result<SyncbackReturn<'sync>> {
    let new_inst = snapshot.new_inst();
    let path = snapshot.path.join(match script_type {
        ScriptType::Server => "init.server.luau",
        ScriptType::Client => "init.client.luau",
        ScriptType::Module => "init.luau",
        _ => anyhow::bail!("syncback is not yet implemented for {script_type:?}"),
    });

    let contents = if let Some(Variant::String(source)) = new_inst.properties.get(&ustr("Source")) {
        source.as_bytes().to_vec()
    } else {
        anyhow::bail!("Scripts must have a `Source` property that is a String")
    };

    let mut dir_syncback = syncback_dir_no_meta(snapshot)?;
    dir_syncback.fs_snapshot.add_file(&path, contents);

    let meta = DirectoryMetadata::from_syncback_snapshot(snapshot, path.clone())?;
    if let Some(mut meta) = meta {
        // Scripts have relatively few properties that we care about, so shifting
        // is fine.
        meta.properties.shift_remove(&ustr("Source"));

        if !meta.is_empty() {
            dir_syncback.fs_snapshot.add_file(
                snapshot.path.join("init.meta.json"),
                serde_json::to_vec_pretty(&meta)
                    .context("could not serialize new init.meta.json")?,
            );
        }
    }

    Ok(dir_syncback)
}

#[cfg(test)]
mod test {
    use super::*;

    use memofs::{InMemoryFs, VfsSnapshot};

    #[test]
    fn class_module_from_vfs() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot("/foo.lua", VfsSnapshot::file("Hello there!"))
            .unwrap();

        let vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_lua(
            &InstanceContext::with_emit_legacy_scripts(Some(true)),
            &vfs,
            Path::new("/foo.lua"),
            "foo",
            ScriptType::Module,
        )
        .unwrap()
        .unwrap();

        insta::with_settings!({ sort_maps => true }, {
            insta::assert_yaml_snapshot!(instance_snapshot);
        });
    }

    #[test]
    fn runcontext_module_from_vfs() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot("/foo.lua", VfsSnapshot::file("Hello there!"))
            .unwrap();

        let vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_lua(
            &InstanceContext::with_emit_legacy_scripts(Some(false)),
            &vfs,
            Path::new("/foo.lua"),
            "foo",
            ScriptType::Module,
        )
        .unwrap()
        .unwrap();

        insta::with_settings!({ sort_maps => true }, {
            insta::assert_yaml_snapshot!(instance_snapshot);
        });
    }

    #[test]
    fn plugin_module_from_vfs() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot("/foo.plugin.lua", VfsSnapshot::file("Hello there!"))
            .unwrap();

        let vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_lua(
            &InstanceContext::with_emit_legacy_scripts(Some(false)),
            &vfs,
            Path::new("/foo.plugin.lua"),
            "foo",
            ScriptType::Plugin,
        )
        .unwrap()
        .unwrap();

        insta::with_settings!({ sort_maps => true }, {
            insta::assert_yaml_snapshot!(instance_snapshot);
        });
    }

    #[test]
    fn class_server_from_vfs() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot("/foo.server.lua", VfsSnapshot::file("Hello there!"))
            .unwrap();

        let vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_lua(
            &InstanceContext::with_emit_legacy_scripts(Some(true)),
            &vfs,
            Path::new("/foo.server.lua"),
            "foo",
            ScriptType::Server,
        )
        .unwrap()
        .unwrap();

        insta::with_settings!({ sort_maps => true }, {
            insta::assert_yaml_snapshot!(instance_snapshot);
        });
    }

    #[test]
    fn runcontext_server_from_vfs() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot("/foo.server.lua", VfsSnapshot::file("Hello there!"))
            .unwrap();

        let vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_lua(
            &InstanceContext::with_emit_legacy_scripts(Some(false)),
            &vfs,
            Path::new("/foo.server.lua"),
            "foo",
            ScriptType::Server,
        )
        .unwrap()
        .unwrap();

        insta::with_settings!({ sort_maps => true }, {
            insta::assert_yaml_snapshot!(instance_snapshot);
        });
    }

    #[test]
    fn class_client_from_vfs() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot("/foo.client.lua", VfsSnapshot::file("Hello there!"))
            .unwrap();

        let vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_lua(
            &InstanceContext::with_emit_legacy_scripts(Some(true)),
            &vfs,
            Path::new("/foo.client.lua"),
            "foo",
            ScriptType::Client,
        )
        .unwrap()
        .unwrap();

        insta::with_settings!({ sort_maps => true }, {
            insta::assert_yaml_snapshot!(instance_snapshot);
        });
    }

    #[test]
    fn runcontext_client_from_vfs() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot("/foo.client.lua", VfsSnapshot::file("Hello there!"))
            .unwrap();

        let vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_lua(
            &InstanceContext::with_emit_legacy_scripts(Some(false)),
            &vfs,
            Path::new("/foo.client.lua"),
            "foo",
            ScriptType::Client,
        )
        .unwrap()
        .unwrap();

        insta::with_settings!({ sort_maps => true }, {
            insta::assert_yaml_snapshot!(instance_snapshot);
        });
    }

    #[test]
    fn init_module_from_vfs() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(
            "/root",
            VfsSnapshot::dir([("init.lua", VfsSnapshot::file("Hello!"))]),
        )
        .unwrap();

        let vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_lua_init(
            &InstanceContext::with_emit_legacy_scripts(Some(true)),
            &vfs,
            Path::new("/root/init.lua"),
            "root",
            ScriptType::Module,
        )
        .unwrap()
        .unwrap();

        insta::with_settings!({ sort_maps => true }, {
            insta::assert_yaml_snapshot!(instance_snapshot);
        });
    }

    #[test]
    fn init_module_from_vfs_with_meta() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(
            "/root",
            VfsSnapshot::dir([
                ("init.lua", VfsSnapshot::file("Hello!")),
                (
                    "init.meta.json",
                    VfsSnapshot::file(r#"{"id": "manually specified"}"#),
                ),
            ]),
        )
        .unwrap();

        let vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_lua_init(
            &InstanceContext::with_emit_legacy_scripts(Some(true)),
            &vfs,
            Path::new("/root/init.lua"),
            "root",
            ScriptType::Module,
        )
        .unwrap()
        .unwrap();

        insta::with_settings!({ sort_maps => true }, {
            insta::assert_yaml_snapshot!(instance_snapshot);
        });
    }

    #[test]
    fn class_module_with_meta() {
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

        let vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_lua(
            &InstanceContext::with_emit_legacy_scripts(Some(true)),
            &vfs,
            Path::new("/foo.lua"),
            "foo",
            ScriptType::Module,
        )
        .unwrap()
        .unwrap();

        insta::with_settings!({ sort_maps => true }, {
            insta::assert_yaml_snapshot!(instance_snapshot);
        });
    }

    #[test]
    fn runcontext_module_with_meta() {
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

        let vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_lua(
            &InstanceContext::with_emit_legacy_scripts(Some(false)),
            &vfs,
            Path::new("/foo.lua"),
            "foo",
            ScriptType::Module,
        )
        .unwrap()
        .unwrap();

        insta::with_settings!({ sort_maps => true }, {
            insta::assert_yaml_snapshot!(instance_snapshot);
        });
    }

    #[test]
    fn class_script_with_meta() {
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

        let vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_lua(
            &InstanceContext::with_emit_legacy_scripts(Some(true)),
            &vfs,
            Path::new("/foo.server.lua"),
            "foo",
            ScriptType::Server,
        )
        .unwrap()
        .unwrap();

        insta::with_settings!({ sort_maps => true }, {
            insta::assert_yaml_snapshot!(instance_snapshot);
        });
    }

    #[test]
    fn runcontext_script_with_meta() {
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

        let vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_lua(
            &InstanceContext::with_emit_legacy_scripts(Some(false)),
            &vfs,
            Path::new("/foo.server.lua"),
            "foo",
            ScriptType::Server,
        )
        .unwrap()
        .unwrap();

        insta::with_settings!({ sort_maps => true }, {
            insta::assert_yaml_snapshot!(instance_snapshot);
        });
    }

    #[test]
    fn class_script_disabled() {
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

        let vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_lua(
            &InstanceContext::with_emit_legacy_scripts(Some(true)),
            &vfs,
            Path::new("/bar.server.lua"),
            "bar",
            ScriptType::Server,
        )
        .unwrap()
        .unwrap();

        insta::with_settings!({ sort_maps => true }, {
            insta::assert_yaml_snapshot!(instance_snapshot);
        });
    }

    #[test]
    fn runcontext_script_disabled() {
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

        let vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_lua(
            &InstanceContext::with_emit_legacy_scripts(Some(false)),
            &vfs,
            Path::new("/bar.server.lua"),
            "bar",
            ScriptType::Server,
        )
        .unwrap()
        .unwrap();

        insta::with_settings!({ sort_maps => true }, {
            insta::assert_yaml_snapshot!(instance_snapshot);
        });
    }
}
