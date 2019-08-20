use std::{
    io,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    File,
    Directory,
}

/// The generic interface that `Imfs` uses to lazily read files from the disk.
/// In tests, it's stubbed out to do different versions of absolutely nothing
/// depending on the test.
pub trait ImfsFetcher {
    fn file_type(&mut self, path: &Path) -> io::Result<FileType>;
    fn read_children(&mut self, path: &Path) -> io::Result<Vec<PathBuf>>;
    fn read_contents(&mut self, path: &Path) -> io::Result<Vec<u8>>;
    fn create_directory(&mut self, path: &Path) -> io::Result<()>;
    fn write_file(&mut self, path: &Path, contents: &[u8]) -> io::Result<()>;
    fn remove(&mut self, path: &Path) -> io::Result<()>;
}