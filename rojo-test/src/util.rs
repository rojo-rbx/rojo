use std::{
    fs, io,
    path::{Path, PathBuf},
    process::Child,
};

use walkdir::WalkDir;

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
    assert!(
        manifest_dir.pop(),
        "Manifest directory did not have a parent"
    );
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

/// Recursively walk a directory and copy each item to the equivalent location
/// in another directory. Equivalent to `cp -r src/* dst`
pub fn copy_recursive(from: &Path, to: &Path) -> io::Result<()> {
    for entry in WalkDir::new(from) {
        let entry = entry?;
        let path = entry.path();
        let new_path = to.join(path.strip_prefix(from).unwrap());

        let file_type = entry.file_type();

        if file_type.is_dir() {
            match fs::create_dir(new_path) {
                Ok(_) => {}
                Err(err) => match err.kind() {
                    io::ErrorKind::AlreadyExists => {}
                    _ => panic!(err),
                },
            }
        } else {
            fs::copy(path, new_path)?;
        }
    }

    Ok(())
}

pub struct KillOnDrop(pub Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
    }
}
