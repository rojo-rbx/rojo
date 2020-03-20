use std::env;
use std::{fs, io, path::{Path, PathBuf}};

use maplit::hashmap;
use memofs::VfsSnapshot;

fn snapshot_from_fs_path(path: &PathBuf) -> io::Result<VfsSnapshot> {
    if path.is_dir() {
        let vfs_entries = fs::read_dir(path)?
            .filter_map(|entry| {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(error) => return Some(Err(error)),
                };

                let file_name = entry.file_name().to_str()
                    .map(str::to_owned)
                    .expect(&format!("Could not get file name from {}", entry.path().display()));

                if file_name.ends_with(".spec.lua") {
                    None
                } else {
                    Some(snapshot_from_fs_path(&entry.path())
                        .map(|snapshot| (file_name, snapshot)))
                }
            });

        let entries: io::Result<Vec<(String, VfsSnapshot)>> = vfs_entries.collect();

        Ok(VfsSnapshot::dir(entries?))
    } else {
        println!("cargo:rerun-if-changed={}", path.display());
        fs::read_to_string(path)
            .map(|content| VfsSnapshot::file(content))
    }
}

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();

    let root_dir = env::var_os("CARGO_MANIFEST_DIR").unwrap();
    let plugin_root = PathBuf::from(root_dir).join("plugin");

    let plugin_modules = plugin_root.join("modules");

    let snapshot = VfsSnapshot::dir(hashmap! {
        "default.project.json" => snapshot_from_fs_path(&plugin_root.join("default.project.json")).unwrap(),
        "fmt" => snapshot_from_fs_path(&plugin_root.join("fmt")).unwrap(),
        "http" => snapshot_from_fs_path(&plugin_root.join("http")).unwrap(),
        "log" => snapshot_from_fs_path(&plugin_root.join("log")).unwrap(),
        "src" => snapshot_from_fs_path(&plugin_root.join("src")).unwrap(),
        "modules" => VfsSnapshot::dir(hashmap! {
            "roact" => VfsSnapshot::dir(hashmap! {
                "src" => snapshot_from_fs_path(&plugin_modules.join("roact").join("src")).unwrap()
            }),
            "promise" => VfsSnapshot::dir(hashmap! {
                "lib" => snapshot_from_fs_path(&plugin_modules.join("promise").join("lib")).unwrap()
            }),
            "t" => VfsSnapshot::dir(hashmap! {
                "lib" => snapshot_from_fs_path(&plugin_modules.join("t").join("lib")).unwrap()
            }),
            "rbx-dom" => VfsSnapshot::dir(hashmap! {
                "rbx_dom_lua" => VfsSnapshot::dir(hashmap! {
                    "src" => snapshot_from_fs_path(&plugin_modules.join("rbx-dom").join("rbx_dom_lua").join("src")).unwrap()
                })
            }),
        }),
    });

    let out_path = Path::new(&out_dir).join("plugin.bincode");
    let out_file = fs::File::create(&out_path).unwrap();

    bincode::serialize_into(out_file, &snapshot).unwrap();
}
