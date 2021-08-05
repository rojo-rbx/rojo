use std::{
    env, io,
    path::{Path, PathBuf},
};

use fs_err as fs;
use fs_err::File;
use maplit::hashmap;
use memofs::VfsSnapshot;

fn snapshot_from_fs_path(path: &Path) -> io::Result<VfsSnapshot> {
    println!("cargo:rerun-if-changed={}", path.display());

    if path.is_dir() {
        let mut children = Vec::new();

        for entry in fs::read_dir(path)? {
            let entry = entry?;

            let file_name = entry.file_name().to_str().unwrap().to_owned();

            // We can skip any TestEZ test files since they aren't necessary for
            // the plugin to run.
            if file_name.ends_with(".spec.lua") {
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

    let root_dir = env::var_os("CARGO_MANIFEST_DIR").unwrap();
    let plugin_root = PathBuf::from(root_dir).join("plugin");

    let plugin_modules = plugin_root.join("modules");

    let snapshot = VfsSnapshot::dir(hashmap! {
        "default.project.json" => snapshot_from_fs_path(&plugin_root.join("default.project.json"))?,
        "fmt" => snapshot_from_fs_path(&plugin_root.join("fmt"))?,
        "http" => snapshot_from_fs_path(&plugin_root.join("http"))?,
        "log" => snapshot_from_fs_path(&plugin_root.join("log"))?,
        "rbx_dom_lua" => snapshot_from_fs_path(&plugin_root.join("rbx_dom_lua"))?,
        "src" => snapshot_from_fs_path(&plugin_root.join("src"))?,
        "modules" => VfsSnapshot::dir(hashmap! {
            "roact" => VfsSnapshot::dir(hashmap! {
                "src" => snapshot_from_fs_path(&plugin_modules.join("roact").join("src"))?
            }),
            "promise" => VfsSnapshot::dir(hashmap! {
                "lib" => snapshot_from_fs_path(&plugin_modules.join("promise").join("lib"))?
            }),
            "t" => VfsSnapshot::dir(hashmap! {
                "lib" => snapshot_from_fs_path(&plugin_modules.join("t").join("lib"))?
            }),
            "flipper" => VfsSnapshot::dir(hashmap! {
                "src" => snapshot_from_fs_path(&plugin_modules.join("flipper").join("src"))?
            }),
        }),
    });

    let out_path = Path::new(&out_dir).join("plugin.bincode");
    let out_file = File::create(&out_path)?;

    bincode::serialize_into(out_file, &snapshot)?;

    Ok(())
}
