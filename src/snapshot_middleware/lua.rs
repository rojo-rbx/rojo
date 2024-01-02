use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
    str,
};

use anyhow::Context;
use memofs::{IoResultExt, Vfs};
use rbx_dom_weak::types::{Enum, Variant};

use crate::{
    resolution::UnresolvedValue,
    snapshot::{InstanceContext, InstanceMetadata, InstanceSnapshot},
    syncback::{FsSnapshot, SyncbackReturn, SyncbackSnapshot},
};

use super::{
    dir::{dir_meta, snapshot_dir_no_meta, syncback_dir_no_meta},
    meta_file::{file_meta, AdjacentMetadata},
    DirectoryMetadata,
};

#[derive(Debug)]
pub enum ScriptType {
    Server,
    Client,
    Module,
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
        .enums
        .get("RunContext")
        .expect("Unable to get RunContext enums!")
        .items;

    let (class_name, run_context) = match (context.emit_legacy_scripts, script_type) {
        (false, ScriptType::Server) => ("Script", run_context_enums.get("Server")),
        (false, ScriptType::Client) => ("Script", run_context_enums.get("Client")),
        (true, ScriptType::Server) => ("Script", run_context_enums.get("Legacy")),
        (true, ScriptType::Client) => ("LocalScript", None),
        (_, ScriptType::Module) => ("ModuleScript", None),
    };

    let contents = vfs.read(path)?;
    let contents_str = str::from_utf8(&contents)
        .with_context(|| format!("File was not valid UTF-8: {}", path.display()))?
        .to_owned();

    let mut properties = HashMap::with_capacity(2);
    properties.insert("Source".to_owned(), contents_str.into());

    if let Some(run_context) = run_context {
        properties.insert(
            "RunContext".to_owned(),
            Enum::from_u32(run_context.to_owned()).into(),
        );
    }

    let meta_path = path.with_file_name(format!("{}.meta.json", name));

    let mut snapshot = InstanceSnapshot::new()
        .name(name)
        .class_name(class_name)
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
    init_snapshot
        .metadata
        .relevant_paths
        .push(init_path.to_owned());

    if let Some(mut meta) = dir_meta(vfs, folder_path)? {
        meta.apply_all(&mut init_snapshot)?;
    }

    Ok(Some(init_snapshot))
}

pub fn syncback_lua<'new, 'old>(
    script_type: ScriptType,
    snapshot: &SyncbackSnapshot<'new, 'old>,
) -> anyhow::Result<SyncbackReturn<'new, 'old>> {
    let new_inst = snapshot.new_inst();

    let mut path = snapshot.parent_path.join(&snapshot.name);
    path.set_extension(match script_type {
        ScriptType::Module => "lua",
        ScriptType::Client => "client.lua",
        ScriptType::Server => "server.lua",
    });
    let contents = if let Some(Variant::String(source)) = new_inst.properties.get("Source") {
        source.as_bytes().to_vec()
    } else {
        anyhow::bail!("Scripts must have a `Source` property that is a String")
    };

    let mut meta = if let Some(meta) = file_meta(snapshot.vfs(), &path, &snapshot.name)? {
        meta
    } else {
        AdjacentMetadata {
            ignore_unknown_instances: None,
            properties: BTreeMap::new(),
            attributes: BTreeMap::new(),
            path: path
                .with_file_name(&snapshot.name)
                .with_extension("meta.json"),
        }
    };
    for (name, value) in snapshot.get_filtered_properties() {
        if name == "Source" {
            continue;
        } else if name == "Attributes" || name == "AttributesSerialize" {
            if let Variant::Attributes(attrs) = value {
                meta.attributes.extend(attrs.iter().map(|(name, value)| {
                    (
                        name.to_string(),
                        UnresolvedValue::FullyQualified(value.clone()),
                    )
                }))
            } else {
                log::error!("Property {name} should be Attributes but is not");
            }
        } else {
            meta.properties.insert(
                name.to_string(),
                UnresolvedValue::from_variant(value.to_owned(), &new_inst.class, name),
            );
        }
    }

    Ok(SyncbackReturn {
        inst_snapshot: InstanceSnapshot::from_instance(new_inst),
        fs_snapshot: FsSnapshot::new().with_added_file(path, contents),
        // Scripts don't have a child!
        children: Vec::new(),
        removed_children: Vec::new(),
    })
}

pub fn syncback_lua_init<'new, 'old>(
    script_type: ScriptType,
    snapshot: &SyncbackSnapshot<'new, 'old>,
) -> anyhow::Result<SyncbackReturn<'new, 'old>> {
    let new_inst = snapshot.new_inst();

    let mut path = snapshot.parent_path.join(&snapshot.name);
    path.push("init");
    path.set_extension(match script_type {
        ScriptType::Module => "lua",
        ScriptType::Client => "client.lua",
        ScriptType::Server => "server.lua",
    });
    let contents = if let Some(Variant::String(source)) = new_inst.properties.get("Source") {
        source.as_bytes().to_vec()
    } else {
        anyhow::bail!("Scripts must have a `Source` property that is a String")
    };

    let dir_syncback = syncback_dir_no_meta(snapshot)?;

    let mut meta = if let Some(dir) = dir_meta(snapshot.vfs(), &path)? {
        dir
    } else {
        DirectoryMetadata {
            ignore_unknown_instances: None,
            class_name: None,
            properties: BTreeMap::new(),
            attributes: BTreeMap::new(),
            path: snapshot
                .parent_path
                .join(&snapshot.name)
                .join("init.meta.json"),
        }
    };
    for (name, value) in snapshot.get_filtered_properties() {
        if name == "Source" {
            continue;
        } else if name == "Attributes" || name == "AttributesSerialize" {
            if let Variant::Attributes(attrs) = value {
                meta.attributes.extend(attrs.iter().map(|(name, value)| {
                    (
                        name.to_string(),
                        UnresolvedValue::FullyQualified(value.clone()),
                    )
                }))
            } else {
                log::error!("Property {name} should be Attributes but is not");
            }
        } else {
            meta.properties.insert(
                name.to_string(),
                UnresolvedValue::from_variant(value.to_owned(), &new_inst.class, name),
            );
        }
    }

    let mut fs_snapshot = FsSnapshot::new();
    fs_snapshot.add_file(path, contents);
    fs_snapshot.merge(dir_syncback.fs_snapshot);

    if !meta.is_empty() {
        fs_snapshot.add_file(
            &meta.path,
            serde_json::to_vec_pretty(&meta).context("could not serialize new init.meta.json")?,
        );
    }

    Ok(SyncbackReturn {
        inst_snapshot: InstanceSnapshot::from_instance(snapshot.new_inst()),
        fs_snapshot,
        children: dir_syncback.children,
        removed_children: dir_syncback.removed_children,
    })
}

#[cfg(test)]
mod test {
    use super::*;

    use maplit::hashmap;
    use memofs::{InMemoryFs, VfsSnapshot};

    #[test]
    fn class_module_from_vfs() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot("/foo.lua", VfsSnapshot::file("Hello there!"))
            .unwrap();

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_lua(
            &InstanceContext::with_emit_legacy_scripts(Some(true)),
            &mut vfs,
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

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_lua(
            &InstanceContext::with_emit_legacy_scripts(Some(false)),
            &mut vfs,
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
    fn class_server_from_vfs() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot("/foo.server.lua", VfsSnapshot::file("Hello there!"))
            .unwrap();

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_lua(
            &InstanceContext::with_emit_legacy_scripts(Some(true)),
            &mut vfs,
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

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_lua(
            &InstanceContext::with_emit_legacy_scripts(Some(false)),
            &mut vfs,
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

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_lua(
            &InstanceContext::with_emit_legacy_scripts(Some(true)),
            &mut vfs,
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

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_lua(
            &InstanceContext::with_emit_legacy_scripts(Some(false)),
            &mut vfs,
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

        let instance_snapshot = snapshot_lua(
            &InstanceContext::with_emit_legacy_scripts(Some(true)),
            &mut vfs,
            Path::new("/root"),
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

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_lua(
            &InstanceContext::with_emit_legacy_scripts(Some(true)),
            &mut vfs,
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

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_lua(
            &InstanceContext::with_emit_legacy_scripts(Some(false)),
            &mut vfs,
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

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_lua(
            &InstanceContext::with_emit_legacy_scripts(Some(true)),
            &mut vfs,
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

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_lua(
            &InstanceContext::with_emit_legacy_scripts(Some(false)),
            &mut vfs,
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

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_lua(
            &InstanceContext::with_emit_legacy_scripts(Some(true)),
            &mut vfs,
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

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_lua(
            &InstanceContext::with_emit_legacy_scripts(Some(false)),
            &mut vfs,
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
