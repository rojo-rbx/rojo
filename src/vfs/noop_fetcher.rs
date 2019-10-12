//! Implements the VFS fetcher interface for a fake filesystem using Rust's
//! std::fs interface.

// This interface is only used for testing, so it's okay if it isn't used.
#![allow(unused)]

use std::{
    io,
    path::{Path, PathBuf},
};

use crossbeam_channel::Receiver;

use super::{
    event::VfsEvent,
    fetcher::{FileType, VfsFetcher},
};

pub struct NoopFetcher;

impl VfsFetcher for NoopFetcher {
    fn file_type(&mut self, _path: &Path) -> io::Result<FileType> {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "NoopFetcher always returns NotFound",
        ))
    }

    fn read_children(&mut self, _path: &Path) -> io::Result<Vec<PathBuf>> {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "NoopFetcher always returns NotFound",
        ))
    }

    fn read_contents(&mut self, _path: &Path) -> io::Result<Vec<u8>> {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "NoopFetcher always returns NotFound",
        ))
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

    fn watch(&mut self, _path: &Path) {}

    fn unwatch(&mut self, _path: &Path) {}

    fn receiver(&self) -> Receiver<VfsEvent> {
        crossbeam_channel::never()
    }
}
