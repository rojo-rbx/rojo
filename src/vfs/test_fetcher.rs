//! Implements the VFS fetcher interface for a fake filesystem that can be
//! mutated and have changes signaled through it.
//!
//! This is useful for testing how things using Vfs react to changed events
//! without relying on the real filesystem implementation, which is very
//! platform-specific.

// This interface is only used for testing, so it's okay if it isn't used.
#![allow(unused)]

use std::{
    io,
    path::{self, Path, PathBuf},
    sync::{Arc, Mutex},
};

use crossbeam_channel::{unbounded, Receiver, Sender};

use crate::path_map::PathMap;

use super::{
    event::VfsEvent,
    fetcher::{FileType, VfsFetcher},
    snapshot::VfsSnapshot,
};

#[derive(Clone)]
pub struct TestFetcherState {
    inner: Arc<Mutex<TestFetcherStateInner>>,
}

impl TestFetcherState {
    pub fn load_snapshot<P: AsRef<Path>>(&self, path: P, snapshot: VfsSnapshot) {
        let mut inner = self.inner.lock().unwrap();
        inner.load_snapshot(path.as_ref().to_path_buf(), snapshot);
    }

    pub fn remove<P: AsRef<Path>>(&self, path: P) {
        let mut inner = self.inner.lock().unwrap();
        inner.remove(path.as_ref());
    }

    pub fn raise_event(&self, event: VfsEvent) {
        let mut inner = self.inner.lock().unwrap();
        inner.raise_event(event);
    }
}

pub enum TestFetcherEntry {
    File(Vec<u8>),
    Dir,
}

struct TestFetcherStateInner {
    entries: PathMap<TestFetcherEntry>,
    sender: Sender<VfsEvent>,
}

impl TestFetcherStateInner {
    fn new(sender: Sender<VfsEvent>) -> Self {
        let mut entries = PathMap::new();
        entries.insert(Path::new("/"), TestFetcherEntry::Dir);

        Self { sender, entries }
    }

    fn load_snapshot(&mut self, path: PathBuf, snapshot: VfsSnapshot) {
        match snapshot {
            VfsSnapshot::File(file) => {
                self.entries
                    .insert(path, TestFetcherEntry::File(file.contents));
            }
            VfsSnapshot::Directory(directory) => {
                self.entries.insert(path.clone(), TestFetcherEntry::Dir);

                for (child_name, child) in directory.children.into_iter() {
                    self.load_snapshot(path.join(child_name), child);
                }
            }
        }
    }

    fn remove(&mut self, path: &Path) {
        self.entries.remove(path);
    }

    fn raise_event(&mut self, event: VfsEvent) {
        self.sender.send(event).unwrap();
    }
}

pub struct TestFetcher {
    state: TestFetcherState,
    receiver: Receiver<VfsEvent>,
}

impl TestFetcher {
    pub fn new() -> (TestFetcherState, Self) {
        let (sender, receiver) = unbounded();

        let state = TestFetcherState {
            inner: Arc::new(Mutex::new(TestFetcherStateInner::new(sender))),
        };

        (state.clone(), Self { receiver, state })
    }
}

impl VfsFetcher for TestFetcher {
    fn file_type(&mut self, path: &Path) -> io::Result<FileType> {
        let inner = self.state.inner.lock().unwrap();

        match inner.entries.get(path) {
            Some(TestFetcherEntry::File(_)) => Ok(FileType::File),
            Some(TestFetcherEntry::Dir) => Ok(FileType::Directory),
            None => Err(io::Error::new(io::ErrorKind::NotFound, "Path not found")),
        }
    }

    fn read_children(&mut self, path: &Path) -> io::Result<Vec<PathBuf>> {
        let inner = self.state.inner.lock().unwrap();

        Ok(inner
            .entries
            .children(path)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Path not found"))?
            .into_iter()
            .map(|path| path.to_path_buf())
            .collect())
    }

    fn read_contents(&mut self, path: &Path) -> io::Result<Vec<u8>> {
        let inner = self.state.inner.lock().unwrap();

        let node = inner.entries.get(path);

        match node {
            Some(TestFetcherEntry::File(contents)) => Ok(contents.clone()),
            Some(TestFetcherEntry::Dir) => Err(io::Error::new(
                io::ErrorKind::Other,
                "Cannot read contents of a directory",
            )),
            None => Err(io::Error::new(io::ErrorKind::NotFound, "Path not found")),
        }
    }

    fn create_directory(&mut self, _path: &Path) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "TestFetcher is not mutable yet",
        ))
    }

    fn write_file(&mut self, _path: &Path, _contents: &[u8]) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "TestFetcher is not mutable yet",
        ))
    }

    fn remove(&mut self, _path: &Path) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "TestFetcher is not mutable yet",
        ))
    }

    fn watch(&mut self, _path: &Path) {}

    fn unwatch(&mut self, _path: &Path) {}

    fn receiver(&self) -> Receiver<VfsEvent> {
        self.receiver.clone()
    }
}
