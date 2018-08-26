use std::env::current_dir;
use std::path::{Path, PathBuf};

/// Turns the path into an absolute one, using the current working directory if
/// necessary.
pub fn canonicalish<T: AsRef<Path>>(value: T) -> PathBuf {
    let cwd = current_dir().unwrap();

    absoluteify(&cwd, value)
}

/// Converts the given path to be absolute if it isn't already using a given
/// root.
fn absoluteify<A, B>(root: A, value: B) -> PathBuf
where
    A: AsRef<Path>,
    B: AsRef<Path>,
{
    let root = root.as_ref();
    let value = value.as_ref();

    if value.is_absolute() {
        PathBuf::from(value)
    } else {
        root.join(value)
    }
}