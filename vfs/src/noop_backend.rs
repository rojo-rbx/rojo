use std::io;
use std::path::Path;

use crate::{Metadata, ReadDir, VfsBackend};

#[non_exhaustive]
pub struct NoopBackend;

impl NoopBackend {
    pub fn new() -> Self {
        Self
    }
}

impl VfsBackend for NoopBackend {
    fn read(&mut self, _path: &Path) -> io::Result<Vec<u8>> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "NoopBackend doesn't do anything",
        ))
    }

    fn write(&mut self, _path: &Path, _data: &[u8]) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "NoopBackend doesn't do anything",
        ))
    }

    fn read_dir(&mut self, _path: &Path) -> io::Result<ReadDir> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "NoopBackend doesn't do anything",
        ))
    }

    fn metadata(&mut self, _path: &Path) -> io::Result<Metadata> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "NoopBackend doesn't do anything",
        ))
    }

    fn watch(&mut self, _path: &Path) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "NoopBackend doesn't do anything",
        ))
    }

    fn unwatch(&mut self, _path: &Path) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "NoopBackend doesn't do anything",
        ))
    }
}
