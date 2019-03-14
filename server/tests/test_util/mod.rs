#![allow(dead_code)]

use std::fs::{create_dir, copy};
use std::path::Path;
use std::io;

use walkdir::WalkDir;

pub mod snapshot;
pub mod tree;

pub fn copy_recursive(from: &Path, to: &Path) -> io::Result<()> {
    for entry in WalkDir::new(from) {
        let entry = entry?;
        let path = entry.path();
        let new_path = to.join(path.strip_prefix(from).unwrap());

        let file_type = entry.file_type();

        if file_type.is_dir() {
            match create_dir(new_path) {
                Ok(_) => {},
                Err(err) => match err.kind() {
                    io::ErrorKind::AlreadyExists => {},
                    _ => panic!(err),
                }
            }
        } else if file_type.is_file() {
            copy(path, new_path)?;
        } else {
            unimplemented!("no symlinks please");
        }
    }

    Ok(())
}