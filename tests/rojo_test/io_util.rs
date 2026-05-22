use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    process::Child,
};

use walkdir::WalkDir;

pub static ROJO_PATH: &str = env!("CARGO_BIN_EXE_rojo");
pub static BUILD_TESTS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/rojo-test/build-tests");
pub static SERVE_TESTS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/rojo-test/serve-tests");
pub static SYNCBACK_TESTS_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/rojo-test/syncback-tests");

pub fn get_working_dir_path() -> PathBuf {
    let mut manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    assert!(
        manifest_dir.pop(),
        "Manifest directory did not have a parent"
    );
    manifest_dir
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
                    _ => return Err(err),
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

        if let Some(mut stdout) = self.0.stdout.take() {
            let mut output = Vec::new();
            let _ = stdout.read_to_end(&mut output);
            print!("{}", String::from_utf8_lossy(&output));
        }

        if let Some(mut stderr) = self.0.stderr.take() {
            let mut output = Vec::new();
            let _ = stderr.read_to_end(&mut output);
            eprint!("{}", String::from_utf8_lossy(&output));
        }
    }
}
