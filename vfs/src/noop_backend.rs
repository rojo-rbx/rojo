use std::io;
use std::path::Path;

use crate::{Metadata, ReadDir, VfsBackend};

pub struct NoopBackend;

impl VfsBackend for NoopBackend {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "NoopBackend doesn't do anything",
        ))
    }

    fn write(&self, path: &Path, data: &[u8]) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "NoopBackend doesn't do anything",
        ))
    }

    fn read_dir(&self, path: &Path) -> io::Result<ReadDir> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "NoopBackend doesn't do anything",
        ))
    }

    fn metadata(&self, path: &Path) -> io::Result<Metadata> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "NoopBackend doesn't do anything",
        ))
    }
}
