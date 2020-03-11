mod in_memory_fs;
mod noop_backend;
mod snapshot;
mod std_backend;

use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

pub use in_memory_fs::InMemoryFs;
pub use noop_backend::NoopBackend;
pub use snapshot::VfsSnapshot;
pub use std_backend::StdBackend;

mod sealed {
    use super::*;

    /// Sealing trait for VfsBackend.
    pub trait Sealed {}

    impl Sealed for NoopBackend {}
    impl Sealed for StdBackend {}
    impl Sealed for InMemoryFs {}
}

/// Trait that transforms `io::Result<T>` into `io::Result<Option<T>>`.
///
/// `Ok(None)` takes the place of IO errors whose `io::ErrorKind` is `NotFound`.
pub trait IoResultExt<T> {
    fn with_not_found(self) -> io::Result<Option<T>>;
}

impl<T> IoResultExt<T> for io::Result<T> {
    fn with_not_found(self) -> io::Result<Option<T>> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(err) => {
                if err.kind() == io::ErrorKind::NotFound {
                    Ok(None)
                } else {
                    Err(err)
                }
            }
        }
    }
}

/// Backend that can be used to create a `Vfs`.
///
/// This trait is sealed and cannot not be implemented outside this crate.
pub trait VfsBackend: sealed::Sealed + Send + 'static {
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

/// Vfs equivalent to [`std::fs::DirEntry`][std::fs::DirEntry].
///
/// [std::fs::DirEntry]: https://doc.rust-lang.org/stable/std/fs/struct.DirEntry.html
pub struct DirEntry {
    pub(crate) path: PathBuf,
}

impl DirEntry {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Vfs equivalent to [`std::fs::ReadDir`][std::fs::ReadDir].
///
/// [std::fs::ReadDir]: https://doc.rust-lang.org/stable/std/fs/struct.ReadDir.html
pub struct ReadDir {
    pub(crate) inner: Box<dyn Iterator<Item = io::Result<DirEntry>>>,
}

impl Iterator for ReadDir {
    type Item = io::Result<DirEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

/// Vfs equivalent to [`std::fs::Metadata`][std::fs::Metadata].
///
/// [std::fs::Metadata]: https://doc.rust-lang.org/stable/std/fs/struct.Metadata.html
#[derive(Debug)]
pub struct Metadata {
    pub(crate) is_file: bool,
}

impl Metadata {
    pub fn is_file(&self) -> bool {
        self.is_file
    }

    pub fn is_dir(&self) -> bool {
        !self.is_file
    }
}

/// Represents an event that a filesystem can raise that might need to be
/// handled.
#[derive(Debug)]
#[non_exhaustive]
pub enum VfsEvent {
    Create(PathBuf),
    Write(PathBuf),
    Remove(PathBuf),
}

/// Contains implementation details of the Vfs, wrapped by `Vfs` and `VfsLock`,
/// the public interfaces to this type.
struct VfsInner {
    backend: Box<dyn VfsBackend>,
}

impl VfsInner {
    fn read<P: AsRef<Path>>(&mut self, path: P) -> io::Result<Arc<Vec<u8>>> {
        let path = path.as_ref();
        let contents = self.backend.read(path)?;
        self.backend.watch(path)?;
        Ok(Arc::new(contents))
    }

    fn write<P: AsRef<Path>, C: AsRef<[u8]>>(&mut self, path: P, contents: C) -> io::Result<()> {
        let path = path.as_ref();
        let contents = contents.as_ref();
        self.backend.write(path, contents)
    }

    fn read_dir<P: AsRef<Path>>(&mut self, path: P) -> io::Result<ReadDir> {
        let path = path.as_ref();
        let dir = self.backend.read_dir(path)?;
        self.backend.watch(path)?;
        Ok(dir)
    }

    fn remove_file<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        let path = path.as_ref();
        let _ = self.backend.unwatch(path);
        self.backend.remove_file(path)
    }

    fn remove_dir_all<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        let path = path.as_ref();
        let _ = self.backend.unwatch(path);
        self.backend.remove_dir_all(path)
    }

    fn metadata<P: AsRef<Path>>(&mut self, path: P) -> io::Result<Metadata> {
        let path = path.as_ref();
        self.backend.metadata(path)
    }

    fn event_receiver(&self) -> crossbeam_channel::Receiver<VfsEvent> {
        self.backend.event_receiver()
    }

    fn commit_event(&mut self, event: &VfsEvent) -> io::Result<()> {
        match event {
            VfsEvent::Remove(path) => {
                let _ = self.backend.unwatch(&path);
            }
            _ => {}
        }

        Ok(())
    }
}

/// A virtual filesystem with a configurable backend.
pub struct Vfs {
    inner: Mutex<VfsInner>,
}

impl Vfs {
    /// Creates a new `Vfs` with the default backend, `StdBackend`.
    pub fn new_default() -> Self {
        Self::new(StdBackend::new())
    }

    /// Creates a new `Vfs` with the given backend.
    pub fn new<B: VfsBackend>(backend: B) -> Self {
        let lock = VfsInner {
            backend: Box::new(backend),
        };

        Self {
            inner: Mutex::new(lock),
        }
    }

    pub fn lock(&self) -> VfsLock<'_> {
        VfsLock {
            inner: self.inner.lock().unwrap(),
        }
    }

    /// Read a file from the VFS, or the underlying backend if it isn't
    /// resident.
    ///
    /// Roughly equivalent to [`std::fs::read`][std::fs::read].
    ///
    /// [std::fs::read]: https://doc.rust-lang.org/stable/std/fs/fn.read.html
    #[inline]
    pub fn read<P: AsRef<Path>>(&self, path: P) -> io::Result<Arc<Vec<u8>>> {
        let path = path.as_ref();
        self.inner.lock().unwrap().read(path)
    }

    /// Write a file to the VFS and the underlying backend.
    ///
    /// Roughly equivalent to [`std::fs::write`][std::fs::write].
    ///
    /// [std::fs::write]: https://doc.rust-lang.org/stable/std/fs/fn.write.html
    #[inline]
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
    #[inline]
    pub fn read_dir<P: AsRef<Path>>(&self, path: P) -> io::Result<ReadDir> {
        let path = path.as_ref();
        self.inner.lock().unwrap().read_dir(path)
    }

    /// Remove a file.
    ///
    /// Roughly equivalent to [`std::fs::remove_file`][std::fs::remove_file].
    ///
    /// [std::fs::remove_file]: https://doc.rust-lang.org/stable/std/fs/fn.remove_file.html
    #[inline]
    pub fn remove_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let path = path.as_ref();
        self.inner.lock().unwrap().remove_file(path)
    }

    /// Remove a directory and all of its descendants.
    ///
    /// Roughly equivalent to [`std::fs::remove_dir_all`][std::fs::remove_dir_all].
    ///
    /// [std::fs::remove_dir_all]: https://doc.rust-lang.org/stable/std/fs/fn.remove_dir_all.html
    #[inline]
    pub fn remove_dir_all<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let path = path.as_ref();
        self.inner.lock().unwrap().remove_dir_all(path)
    }

    /// Query metadata about the given path.
    ///
    /// Roughly equivalent to [`std::fs::metadata`][std::fs::metadata].
    ///
    /// [std::fs::metadata]: https://doc.rust-lang.org/stable/std/fs/fn.metadata.html
    #[inline]
    pub fn metadata<P: AsRef<Path>>(&self, path: P) -> io::Result<Metadata> {
        let path = path.as_ref();
        self.inner.lock().unwrap().metadata(path)
    }

    /// Retrieve a handle to the event receiver for this `Vfs`.
    #[inline]
    pub fn event_receiver(&self) -> crossbeam_channel::Receiver<VfsEvent> {
        self.inner.lock().unwrap().event_receiver()
    }

    /// Commit an event to this `Vfs`.
    #[inline]
    pub fn commit_event(&self, event: &VfsEvent) -> io::Result<()> {
        self.inner.lock().unwrap().commit_event(event)
    }
}

/// A locked handle to a `Vfs`, created by `Vfs::lock`.
pub struct VfsLock<'a> {
    inner: MutexGuard<'a, VfsInner>,
}

impl VfsLock<'_> {
    /// Read a file from the VFS, or the underlying backend if it isn't
    /// resident.
    ///
    /// Roughly equivalent to [`std::fs::read`][std::fs::read].
    ///
    /// [std::fs::read]: https://doc.rust-lang.org/stable/std/fs/fn.read.html
    #[inline]
    pub fn read<P: AsRef<Path>>(&mut self, path: P) -> io::Result<Arc<Vec<u8>>> {
        let path = path.as_ref();
        self.inner.read(path)
    }

    /// Write a file to the VFS and the underlying backend.
    ///
    /// Roughly equivalent to [`std::fs::write`][std::fs::write].
    ///
    /// [std::fs::write]: https://doc.rust-lang.org/stable/std/fs/fn.write.html
    #[inline]
    pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(
        &mut self,
        path: P,
        contents: C,
    ) -> io::Result<()> {
        let path = path.as_ref();
        let contents = contents.as_ref();
        self.inner.write(path, contents)
    }

    /// Read all of the children of a directory.
    ///
    /// Roughly equivalent to [`std::fs::read_dir`][std::fs::read_dir].
    ///
    /// [std::fs::read_dir]: https://doc.rust-lang.org/stable/std/fs/fn.read_dir.html
    #[inline]
    pub fn read_dir<P: AsRef<Path>>(&mut self, path: P) -> io::Result<ReadDir> {
        let path = path.as_ref();
        self.inner.read_dir(path)
    }

    /// Remove a file.
    ///
    /// Roughly equivalent to [`std::fs::remove_file`][std::fs::remove_file].
    ///
    /// [std::fs::remove_file]: https://doc.rust-lang.org/stable/std/fs/fn.remove_file.html
    #[inline]
    pub fn remove_file<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        let path = path.as_ref();
        self.inner.remove_file(path)
    }

    /// Remove a directory and all of its descendants.
    ///
    /// Roughly equivalent to [`std::fs::remove_dir_all`][std::fs::remove_dir_all].
    ///
    /// [std::fs::remove_dir_all]: https://doc.rust-lang.org/stable/std/fs/fn.remove_dir_all.html
    #[inline]
    pub fn remove_dir_all<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        let path = path.as_ref();
        self.inner.remove_dir_all(path)
    }

    /// Query metadata about the given path.
    ///
    /// Roughly equivalent to [`std::fs::metadata`][std::fs::metadata].
    ///
    /// [std::fs::metadata]: https://doc.rust-lang.org/stable/std/fs/fn.metadata.html
    #[inline]
    pub fn metadata<P: AsRef<Path>>(&mut self, path: P) -> io::Result<Metadata> {
        let path = path.as_ref();
        self.inner.metadata(path)
    }

    /// Retrieve a handle to the event receiver for this `Vfs`.
    #[inline]
    pub fn event_receiver(&self) -> crossbeam_channel::Receiver<VfsEvent> {
        self.inner.event_receiver()
    }

    /// Commit an event to this `Vfs`.
    #[inline]
    pub fn commit_event(&mut self, event: &VfsEvent) -> io::Result<()> {
        self.inner.commit_event(event)
    }
}
