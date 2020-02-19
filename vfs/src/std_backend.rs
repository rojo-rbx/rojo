use std::fs;
use std::io;
use std::path::Path;

use crate::{DirEntry, Metadata, ReadDir, VfsBackend};

pub struct StdBackend;

impl VfsBackend for StdBackend {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        fs::read(path)
    }

    fn write(&self, path: &Path, data: &[u8]) -> io::Result<()> {
        fs::write(path, data)
    }

    fn read_dir(&self, path: &Path) -> io::Result<ReadDir> {
        let inner = fs::read_dir(path)?.map(|entry| {
            Ok(DirEntry {
                path: entry?.path(),
            })
        });

        Ok(ReadDir {
            inner: Box::new(inner),
        })
    }

    fn metadata(&self, path: &Path) -> io::Result<Metadata> {
        let inner = fs::metadata(path)?;

        Ok(Metadata {
            is_file: inner.is_file(),
        })
    }
}
