//! Implements the IMFS fetcher interface for the real filesystem using Rust's
//! std::fs interface.

use std::{
    io,
    fs,
    path::Path,
};

use super::interface::{ImfsFetcher, ImfsItem, ImfsDirectory, ImfsFile};

pub struct RealFetcher;

impl ImfsFetcher for RealFetcher {
    fn read_item(&self, path: impl AsRef<Path>) -> io::Result<ImfsItem> {
        let metadata = fs::metadata(path.as_ref())?;

        if metadata.is_file() {
            Ok(ImfsItem::File(ImfsFile {
                path: path.as_ref().to_path_buf(),
                contents: None,
            }))
        } else {
            Ok(ImfsItem::Directory(ImfsDirectory {
                path: path.as_ref().to_path_buf(),
                children_enumerated: false,
            }))
        }
    }

    fn read_children(&self, path: impl AsRef<Path>) -> io::Result<Vec<ImfsItem>> {
        let mut result = Vec::new();

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            result.push(self.read_item(entry.path())?);
        }

        Ok(result)
    }

    fn read_contents(&self, path: impl AsRef<Path>) -> io::Result<Vec<u8>> {
        fs::read(path)
    }

    fn create_directory(&self, path: impl AsRef<Path>) -> io::Result<()> {
        match fs::create_dir(path) {
            Ok(_) => Ok(()),
            Err(ref err) if err.kind() == io::ErrorKind::AlreadyExists => Ok(()),
            Err(err) => Err(err)
        }
    }

    fn write_contents(&self, path: impl AsRef<Path>, contents: &[u8]) -> io::Result<()> {
        fs::write(path, contents)
    }

    fn remove(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let metadata = fs::metadata(path.as_ref())?;

        if metadata.is_file() {
            fs::remove_file(path)
        } else {
            fs::remove_dir_all(path)
        }
    }
}