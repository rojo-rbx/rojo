use std::io;
use std::path::Path;

use crate::{Metadata, ReadDir, VfsBackend, VfsEvent};

/// `VfsBackend` that reads from an in-memory filesystem.
#[non_exhaustive]
pub struct MemoryBackend;

impl MemoryBackend {
    pub fn new() -> Self {
        Self
    }
}

impl VfsBackend for MemoryBackend {
    fn read(&mut self, _path: &Path) -> io::Result<Vec<u8>> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "MemoryBackend doesn't do anything",
        ))
    }

    fn write(&mut self, _path: &Path, _data: &[u8]) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "MemoryBackend doesn't do anything",
        ))
    }

    fn read_dir(&mut self, _path: &Path) -> io::Result<ReadDir> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "MemoryBackend doesn't do anything",
        ))
    }

    fn remove_file(&mut self, _path: &Path) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "MemoryBackend doesn't do anything",
        ))
    }

    fn remove_dir_all(&mut self, _path: &Path) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "MemoryBackend doesn't do anything",
        ))
    }

    fn metadata(&mut self, _path: &Path) -> io::Result<Metadata> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "MemoryBackend doesn't do anything",
        ))
    }

    fn event_receiver(&self) -> crossbeam_channel::Receiver<VfsEvent> {
        crossbeam_channel::never()
    }

    fn watch(&mut self, _path: &Path) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "MemoryBackend doesn't do anything",
        ))
    }

    fn unwatch(&mut self, _path: &Path) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "MemoryBackend doesn't do anything",
        ))
    }
}
