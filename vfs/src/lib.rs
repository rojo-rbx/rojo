mod noop_backend;
mod std_backend;

use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub use noop_backend::NoopBackend;
pub use std_backend::StdBackend;

mod sealed {
    use super::*;

    pub trait Sealed {}

    impl Sealed for NoopBackend {}
    impl Sealed for StdBackend {}
}

/// Backend that can be used to create a
///
/// This trait is sealed and cannot not be implemented outside this crate.
pub trait VfsBackend: sealed::Sealed {
    fn read(&mut self, path: &Path) -> io::Result<Vec<u8>>;
    fn write(&mut self, path: &Path, data: &[u8]) -> io::Result<()>;
    fn read_dir(&mut self, path: &Path) -> io::Result<ReadDir>;
    fn metadata(&mut self, path: &Path) -> io::Result<Metadata>;
    fn remove_file(&mut self, path: &Path) -> io::Result<()>;
    fn remove_dir_all(&mut self, path: &Path) -> io::Result<()>;

    fn event_receiver(&self) -> crossbeam_channel::Receiver<VfsEvent>;
    fn watch(&mut self, path: &Path) -> io::Result<()>;
    fn unwatch(&mut self, path: &Path) -> io::Result<()>;
}

pub struct DirEntry {
    path: PathBuf,
}

impl DirEntry {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

pub struct ReadDir {
    inner: Box<dyn Iterator<Item = io::Result<DirEntry>>>,
}

impl Iterator for ReadDir {
    type Item = io::Result<DirEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

#[derive(Debug)]
pub struct Metadata {
    is_file: bool,
}

impl Metadata {
    pub fn is_file(&self) -> bool {
        self.is_file
    }

    pub fn is_dir(&self) -> bool {
        !self.is_file
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub enum VfsEvent {
    Create(PathBuf),
    Write(PathBuf),
    Remove(PathBuf),
}

struct VfsLock {
    backend: Box<dyn VfsBackend>,
}

impl VfsLock {
    pub fn read<P: AsRef<Path>>(&mut self, path: P) -> io::Result<Arc<Vec<u8>>> {
        let path = path.as_ref();
        let contents = self.backend.read(path)?;
        self.backend.watch(path)?;
        Ok(Arc::new(contents))
    }

    pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(
        &mut self,
        path: P,
        contents: C,
    ) -> io::Result<()> {
        let path = path.as_ref();
        let contents = contents.as_ref();
        self.backend.write(path, contents)
    }

    pub fn read_dir<P: AsRef<Path>>(&mut self, path: P) -> io::Result<ReadDir> {
        let path = path.as_ref();
        let dir = self.backend.read_dir(path)?;
        self.backend.watch(path)?;
        Ok(dir)
    }

    pub fn event_receiver(&self) -> crossbeam_channel::Receiver<VfsEvent> {
        self.backend.event_receiver()
    }

    pub fn commit_event(&mut self, _event: &VfsEvent) -> io::Result<()> {
        // Because we hold no state, there's no cache to update here.
        Ok(())
    }
}

/// A virtual filesystem with a configurable backend.
pub struct Vfs {
    inner: Mutex<VfsLock>,
}

impl Vfs {
    /// Creates a new `Vfs` with the default backend, `StdBackend`.
    pub fn new_default() -> Self {
        Self::new(StdBackend::new())
    }

    /// Creates a new `Vfs` with the given backend.
    pub fn new<B: VfsBackend + 'static>(backend: B) -> Self {
        let lock = VfsLock {
            backend: Box::new(backend),
        };

        Self {
            inner: Mutex::new(lock),
        }
    }

    /// Read a file from the VFS, or the underlying backend if it isn't
    /// resident.
    ///
    /// Roughly equivalent to [`std::fs::read`][std::fs::read].
    ///
    /// [std::fs::read]: https://doc.rust-lang.org/stable/std/fs/fn.read.html
    pub fn read<P: AsRef<Path>>(&self, path: P) -> io::Result<Arc<Vec<u8>>> {
        let path = path.as_ref();
        self.inner.lock().unwrap().read(path)
    }

    /// Write a file to the VFS and the underlying backend.
    ///
    /// Roughly equivalent to [`std::fs::write`][std::fs::write].
    ///
    /// [std::fs::write]: https://doc.rust-lang.org/stable/std/fs/fn.write.html
    pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(&self, path: P, contents: C) -> io::Result<()> {
        let path = path.as_ref();
        let contents = contents.as_ref();
        self.inner.lock().unwrap().write(path, contents)
    }

    /// Read all of the children of a directory.
    ///
    /// Roughly equivalent to [`std::fs::read_dir`][std::fs::read_dir].
    ///
    /// [std::fs::read_dir]: https://doc.rust-lang.org/stable/std/fs/fn.read_dir.html
    pub fn read_dir<P: AsRef<Path>>(&self, path: P) -> io::Result<ReadDir> {
        let path = path.as_ref();
        self.inner.lock().unwrap().read_dir(path)
    }

    /// Retrieve a handle to the event receiver for this `Vfs`.
    pub fn event_receiver(&self) -> crossbeam_channel::Receiver<VfsEvent> {
        self.inner.lock().unwrap().event_receiver()
    }

    /// Commit an event to this `Vfs`.
    pub fn commit_event(&self, event: &VfsEvent) -> io::Result<()> {
        self.inner.lock().unwrap().commit_event(event)
    }
}
