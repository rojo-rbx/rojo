//! Implements the IMFS fetcher interface for the real filesystem using Rust's
//! std::fs interface.

use std::{
    io,
    fs,
    path::{Path, PathBuf},
};

use super::{
    imfs::{ImfsItem, ImfsDirectory, ImfsFile},
    fetcher::ImfsFetcher,
};

pub struct RealFetcher;

impl ImfsFetcher for RealFetcher {
    fn read_item(&mut self, path: &Path) -> io::Result<ImfsItem> {
        let metadata = fs::metadata(path)?;

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

    fn read_children(&mut self, path: &Path) -> io::Result<Vec<PathBuf>> {
        let mut result = Vec::new();

        let iter = fs::read_dir(path)?;

        for entry in iter {
            result.push(entry?.path());
        }

        Ok(result)
    }

    fn read_contents(&mut self, path: &Path) -> io::Result<Vec<u8>> {
        fs::read(path)
    }

    fn create_directory(&mut self, path: &Path) -> io::Result<()> {
        fs::create_dir(path)
    }

    fn write_file(&mut self, path: &Path, contents: &[u8]) -> io::Result<()> {
        fs::write(path, contents)
    }

    fn remove(&mut self, path: &Path) -> io::Result<()> {
        let metadata = fs::metadata(path)?;

        if metadata.is_file() {
            fs::remove_file(path)
        } else {
            fs::remove_dir_all(path)
        }
    }
}