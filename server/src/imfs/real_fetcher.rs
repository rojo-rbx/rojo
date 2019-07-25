//! Implements the IMFS fetcher interface for the real filesystem using Rust's
//! std::fs interface.

use std::{
    io,
    fs,
    path::Path,
};

use super::{
    error::{FsError, FsResult},
    interface::{ImfsFetcher, ImfsItem, ImfsDirectory, ImfsFile},
};

pub struct RealFetcher;

impl ImfsFetcher for RealFetcher {
    fn read_item(&mut self, path: &Path) -> FsResult<ImfsItem> {
        let metadata = fs::metadata(path)
            .map_err(|err| FsError::new(err, path))?;

        if metadata.is_file() {
            Ok(ImfsItem::File(ImfsFile {
                path: path.to_path_buf(),
                contents: None,
            }))
        } else {
            Ok(ImfsItem::Directory(ImfsDirectory {
                path: path.to_path_buf(),
                children_enumerated: false,
            }))
        }
    }

    fn read_children(&mut self, path: &Path) -> FsResult<Vec<ImfsItem>> {
        let mut result = Vec::new();

        let iter = fs::read_dir(path)
            .map_err(|err| FsError::new(err, path))?;

        for entry in iter {
            let entry = entry
                .map_err(|err| FsError::new(err, path))?;

            result.push(self.read_item(&entry.path())?);
        }

        Ok(result)
    }

    fn read_contents(&mut self, path: &Path) -> FsResult<Vec<u8>> {
        fs::read(path)
            .map_err(|err| FsError::new(err, path))
    }

    fn create_directory(&mut self, path: &Path) -> FsResult<()> {
        match fs::create_dir(path) {
            Ok(_) => Ok(()),
            Err(ref err) if err.kind() == io::ErrorKind::AlreadyExists => Ok(()),
            Err(err) => Err(FsError::new(err, path))
        }
    }

    fn write_contents(&mut self, path: &Path, contents: &[u8]) -> FsResult<()> {
        fs::write(path, contents)
            .map_err(|err| FsError::new(err, path))
    }

    fn remove(&mut self, path: &Path) -> FsResult<()> {
        let metadata = fs::metadata(path)
            .map_err(|err| FsError::new(err, path))?;

        if metadata.is_file() {
            fs::remove_file(path)
                .map_err(|err| FsError::new(err, path))
        } else {
            fs::remove_dir_all(path)
                .map_err(|err| FsError::new(err, path))
        }
    }
}