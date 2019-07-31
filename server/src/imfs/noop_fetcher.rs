//! Implements the IMFS fetcher interface for a fake filesystem using Rust's
//! std::fs interface.

use std::{
    io,
    path::Path,
};

use super::{
    interface::{ImfsFetcher, ImfsItem},
};

pub struct NoopFetcher;

impl ImfsFetcher for NoopFetcher {
    fn read_item(&mut self, _path: &Path) -> io::Result<ImfsItem> {
        Err(io::Error::new(io::ErrorKind::NotFound, "no-op"))
    }

    fn read_children(&mut self, _path: &Path) -> io::Result<Vec<ImfsItem>> {
        Err(io::Error::new(io::ErrorKind::NotFound, "no-op"))
    }

    fn read_contents(&mut self, _path: &Path) -> io::Result<Vec<u8>> {
        Err(io::Error::new(io::ErrorKind::NotFound, "no-op"))
    }

    fn create_directory(&mut self, _path: &Path) -> io::Result<()> {
        Ok(())
    }

    fn write_file(&mut self, _path: &Path, _contents: &[u8]) -> io::Result<()> {
        Ok(())
    }

    fn remove(&mut self, _path: &Path) -> io::Result<()> {
        Ok(())
    }
}