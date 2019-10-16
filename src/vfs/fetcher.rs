use std::{
    io,
    path::{Path, PathBuf},
};

use crossbeam_channel::Receiver;

use super::event::VfsEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    File,
    Directory,
}

/// The generic interface that `Vfs` uses to lazily read files from the disk.
/// In tests, it's stubbed out to do different versions of absolutely nothing
/// depending on the test.
pub trait VfsFetcher {
    fn file_type(&self, path: &Path) -> io::Result<FileType>;
    fn read_children(&self, path: &Path) -> io::Result<Vec<PathBuf>>;
    fn read_contents(&self, path: &Path) -> io::Result<Vec<u8>>;

    fn create_directory(&self, path: &Path) -> io::Result<()>;
    fn write_file(&self, path: &Path, contents: &[u8]) -> io::Result<()>;
    fn remove(&self, path: &Path) -> io::Result<()>;

    fn receiver(&self) -> Receiver<VfsEvent>;

    fn watch(&self, _path: &Path) {}
    fn unwatch(&self, _path: &Path) {}

    /// A method intended for debugging what paths the fetcher is watching.
    fn watched_paths(&self) -> Vec<PathBuf> {
        Vec::new()
    }
}
