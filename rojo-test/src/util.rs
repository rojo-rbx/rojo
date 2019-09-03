use std::path::{Path, PathBuf};

pub fn get_rojo_path() -> PathBuf {
    let working_dir = get_working_dir_path();

    let mut exe_path = working_dir.join("target/debug/rojo");
    if cfg!(windows) {
        exe_path.set_extension("exe");
    }

    exe_path
}

pub fn get_working_dir_path() -> PathBuf {
    let mut manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    assert!(manifest_dir.pop(), "Manifest directory did not have a parent");
    manifest_dir
}

pub fn get_build_tests_path() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("build-tests")
}

pub fn get_serve_tests_path() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("serve-tests")
}