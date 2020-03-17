use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use maplit::hashmap;
use memofs::VfsSnapshot;

fn snapshot_from_fs_path(path: &PathBuf) -> Result<VfsSnapshot, PathBuf> {
    if path.is_dir() {
        let entries: Result<Vec<fs::DirEntry>, PathBuf> = fs::read_dir(path)
            .map_err(|_| path.to_owned())?
            .map(|entry| entry.map_err(|_| path.to_owned()))
            .into_iter()
            .collect();

        let vfs_entries: Result<Vec<(String, VfsSnapshot)>, PathBuf> = entries?
            .iter()
            .filter(|entry| !entry.path().ends_with(".spec.lua"))
            .map(|entry| {
                let path = entry.path();

                path.file_name()
                    .and_then(|file_name| file_name.to_str())
                    .ok_or(path.to_owned())
                    .and_then(|file_name| {
                        snapshot_from_fs_path(&path).map(|snapshot| (file_name.to_owned(), snapshot))
                    })
            })
            .into_iter()
            .collect();

        Ok(VfsSnapshot::dir(vfs_entries?))
    } else {
        println!("cargo:rerun-if-changed={}", path.to_string_lossy());
        fs::read_to_string(path)
            .ok()
            .map(|content| VfsSnapshot::file(content))
            .ok_or(path.to_owned())
    }
}

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();

    let plugin_root = PathBuf::from("plugin");

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

    println!("cargo:rerun-if-changed=build.rs");

    let out_path = Path::new(&out_dir).join("plugin.bincode");
    let out_file = fs::File::create(&out_path).unwrap();

    bincode::serialize_into(out_file, &snapshot).unwrap();
}
