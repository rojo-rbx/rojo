use std::{
    env, io,
    path::{Path, PathBuf},
};

use fs_err as fs;
use fs_err::File;
use maplit::hashmap;
use memofs::VfsSnapshot;
use semver::Version;

fn snapshot_from_fs_path(path: &Path) -> io::Result<VfsSnapshot> {
    println!("cargo:rerun-if-changed={}", path.display());

    if path.is_dir() {
        let mut children = Vec::new();

        for entry in fs::read_dir(path)? {
            let entry = entry?;

            let file_name = entry.file_name().to_str().unwrap().to_owned();

            if file_name.starts_with(".git") {
                continue;
            }

            // We can skip any TestEZ test files since they aren't necessary for
            // the plugin to run.
            if file_name.ends_with(".spec.lua") || file_name.ends_with(".spec.luau") {
                continue;
            }

            let child_snapshot = snapshot_from_fs_path(&entry.path())?;
            children.push((file_name, child_snapshot));
        }

        Ok(VfsSnapshot::dir(children))
    } else {
        let content = fs::read_to_string(path)?;

        Ok(VfsSnapshot::file(content))
    }
}

fn main() -> Result<(), anyhow::Error> {
    let out_dir = env::var_os("OUT_DIR").unwrap();

    let root_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let plugin_dir = root_dir.join("plugin");
    let templates_dir = root_dir.join("assets").join("project-templates");

    let our_version = Version::parse(env::var_os("CARGO_PKG_VERSION").unwrap().to_str().unwrap())?;
    let plugin_version =
        Version::parse(fs::read_to_string(plugin_dir.join("Version.txt"))?.trim())?;

    assert_eq!(
        our_version, plugin_version,
        "plugin version does not match Cargo version"
    );

    let template_snapshot = snapshot_from_fs_path(&templates_dir)?;

    let plugin_snapshot = VfsSnapshot::dir(hashmap! {
        "default.project.json" => snapshot_from_fs_path(&root_dir.join("plugin.project.json"))?,
        "plugin" => VfsSnapshot::dir(hashmap! {
            "fmt" => snapshot_from_fs_path(&plugin_dir.join("fmt"))?,
            "http" => snapshot_from_fs_path(&plugin_dir.join("http"))?,
            "log" => snapshot_from_fs_path(&plugin_dir.join("log"))?,
            "rbx_dom_lua" => snapshot_from_fs_path(&plugin_dir.join("rbx_dom_lua"))?,
            "src" => snapshot_from_fs_path(&plugin_dir.join("src"))?,
            "Packages" => snapshot_from_fs_path(&plugin_dir.join("Packages"))?,
            "Version.txt" => snapshot_from_fs_path(&plugin_dir.join("Version.txt"))?,
        }),
    });

    let template_file = File::create(Path::new(&out_dir).join("templates.bincode"))?;
    let plugin_file = File::create(Path::new(&out_dir).join("plugin.bincode"))?;

    bincode::serialize_into(plugin_file, &plugin_snapshot)?;
    bincode::serialize_into(template_file, &template_snapshot)?;

    println!("cargo:rerun-if-changed=build/windows/rojo-manifest.rc");
    println!("cargo:rerun-if-changed=build/windows/rojo.manifest");
    embed_resource::compile("build/windows/rojo-manifest.rc");

    Ok(())
}
