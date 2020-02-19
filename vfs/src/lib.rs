mod noop_backend;
mod std_backend;

use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub use noop_backend::NoopBackend;
pub use std_backend::StdBackend;

pub trait VfsBackend {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;
    fn write(&self, path: &Path, data: &[u8]) -> io::Result<()>;
    fn read_dir(&self, path: &Path) -> io::Result<ReadDir>;
    fn metadata(&self, path: &Path) -> io::Result<Metadata>;
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

struct VfsLock {
    data: (),
    backend: Box<dyn VfsBackend>,
}

impl VfsLock {
    pub fn read<P: AsRef<Path>>(&mut self, path: P) -> io::Result<Arc<Vec<u8>>> {
        let path = path.as_ref();
        let contents = self.backend.read(path)?;
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
        self.backend.read_dir(path)
    }
}

pub struct Vfs {
    inner: Mutex<VfsLock>,
}

impl Vfs {
    pub fn new<B: VfsBackend + 'static>(backend: B) -> Self {
        let lock = VfsLock {
            data: (),
            backend: Box::new(backend),
        };

        Self {
            inner: Mutex::new(lock),
        }
    }

    pub fn read<P: AsRef<Path>>(&self, path: P) -> io::Result<Arc<Vec<u8>>> {
        let path = path.as_ref();
        let mut inner = self.inner.lock().unwrap();
        inner.read(path)
    }

    pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(&self, path: P, contents: C) -> io::Result<()> {
        let path = path.as_ref();
        let contents = contents.as_ref();
        let mut inner = self.inner.lock().unwrap();
        inner.write(path, contents)
    }

    pub fn read_dir<P: AsRef<Path>>(&self, path: P) -> io::Result<ReadDir> {
        let path = path.as_ref();
        let mut inner = self.inner.lock().unwrap();
        inner.read_dir(path)
    }
}
