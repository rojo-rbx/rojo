use std::fs;
use std::io;
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use notify::{watcher, DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};

use crate::{DirEntry, Metadata, ReadDir, VfsBackend};

pub struct StdBackend {
    watcher: RecommendedWatcher,
    watcher_receiver: mpsc::Receiver<DebouncedEvent>,
}

impl StdBackend {
    pub fn new() -> StdBackend {
        let (tx, rx) = mpsc::channel();
        let watcher = watcher(tx, Duration::from_millis(50)).unwrap();

        Self {
            watcher,
            watcher_receiver: rx,
        }
    }
}

impl VfsBackend for StdBackend {
    fn read(&mut self, path: &Path) -> io::Result<Vec<u8>> {
        fs::read(path)
    }

    fn write(&mut self, path: &Path, data: &[u8]) -> io::Result<()> {
        fs::write(path, data)
    }

    fn read_dir(&mut self, path: &Path) -> io::Result<ReadDir> {
        let inner = fs::read_dir(path)?.map(|entry| {
            Ok(DirEntry {
                path: entry?.path(),
            })
        });

        Ok(ReadDir {
            inner: Box::new(inner),
        })
    }

    fn metadata(&mut self, path: &Path) -> io::Result<Metadata> {
        let inner = fs::metadata(path)?;

        Ok(Metadata {
            is_file: inner.is_file(),
        })
    }

    fn watch(&mut self, path: &Path) -> io::Result<()> {
        self.watcher
            .watch(path, RecursiveMode::NonRecursive)
            .map_err(|inner| io::Error::new(io::ErrorKind::Other, inner))
    }

    fn unwatch(&mut self, path: &Path) -> io::Result<()> {
        self.watcher
            .unwatch(path)
            .map_err(|inner| io::Error::new(io::ErrorKind::Other, inner))
    }
}
