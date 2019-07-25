use std::path::{Path, PathBuf};

use crate::path_map::PathMap;

use super::error::FsResult;

/// The generic interface that `Imfs` uses to lazily read files from the disk.
/// In tests, it's stubbed out to do different versions of absolutely nothing
/// depending on the test.
pub trait ImfsFetcher {
    fn read_item(&mut self, path: &Path) -> FsResult<ImfsItem>;
    fn read_children(&mut self, path: &Path) -> FsResult<Vec<ImfsItem>>;
    fn read_contents(&mut self, path: &Path) -> FsResult<Vec<u8>>;
    fn create_directory(&mut self, path: &Path) -> FsResult<()>;
    fn write_file(&mut self, path: &Path, contents: &[u8]) -> FsResult<()>;
    fn remove(&mut self, path: &Path) -> FsResult<()>;
}

/// An in-memory filesystem that can be incrementally populated and updated as
/// filesystem modification events occur.
///
/// All operations on the `Imfs` are lazy and do I/O as late as they can to
/// avoid reading extraneous files or directories from the disk. This means that
/// they all take `self` mutably, and means that it isn't possible to hold
/// references to the internal state of the Imfs while traversing it!
///
/// Most operations return `ImfsEntry` objects to work around this, which is
/// effectively a index into the `Imfs`.
pub struct Imfs<F> {
    inner: PathMap<ImfsItem>,
    fetcher: F,
}

impl<F: ImfsFetcher> Imfs<F> {
    /// Tells whether the given path, if it were loaded, would be loaded if it
    /// existed.
    ///
    /// Returns true if the path is loaded or if its parent is loaded, is a
    /// directory, and is marked as having been enumerated before.
    ///
    /// This idea corresponds to whether a file change event should result in
    /// tangible changes to the in-memory filesystem. If a path would be
    /// resident, we need to read it, and if its contents were known before, we
    /// need to update them.
    fn would_be_resident(&self, path: &Path) -> bool {
        if self.inner.contains_key(path) {
            return true;
        }

        if let Some(parent) = path.parent() {
            if let Some(ImfsItem::Directory(dir)) = self.inner.get(parent) {
                return !dir.children_enumerated;
            }
        }

        false
    }

    /// Attempts to read the path into the `Imfs` if it doesn't exist.
    ///
    /// This does not necessitate that file contents or directory children will
    /// be read. Depending on the `ImfsFetcher` implementation that the `Imfs`
    /// is using, this call may read exactly only the given path and no more.
    fn read_if_not_exists(&mut self, path: &Path) -> FsResult<()> {
        if !self.inner.contains_key(path) {
            let item = self.fetcher.read_item(path)?;
            self.inner.insert(path.to_path_buf(), item);
        }

        Ok(())
    }

    pub fn raise_file_change(&mut self, path: impl AsRef<Path>) -> FsResult<()> {
        if !self.would_be_resident(path.as_ref()) {
            return Ok(());
        }

        unimplemented!();
    }

    pub fn raise_file_removed(&mut self, path: impl AsRef<Path>) -> FsResult<()> {
        if !self.would_be_resident(path.as_ref()) {
            return Ok(());
        }

        unimplemented!();
    }

    pub fn get(&mut self, path: impl AsRef<Path>) -> Option<ImfsEntry> {
        self.read_if_not_exists(path.as_ref())
            .expect("TODO: Handle this error");

        let item = self.inner.get(path.as_ref())?;

        let is_file = match item {
            ImfsItem::File(_) => true,
            ImfsItem::Directory(_) => false,
        };

        Some(ImfsEntry {
            path: item.path().to_path_buf(),
            is_file,
        })
    }

    pub fn get_contents(&mut self, path: impl AsRef<Path>) -> Option<&[u8]> {
        self.read_if_not_exists(path.as_ref())
            .expect("TODO: Handle this error");

        match self.inner.get_mut(path.as_ref())? {
            ImfsItem::File(file) => {
                if file.contents.is_none() {
                    file.contents = Some(self.fetcher.read_contents(path.as_ref())
                        .expect("TODO: Handle this error"));
                }

                Some(file.contents.as_ref().unwrap())
            }
            ImfsItem::Directory(_) => None
        }
    }

    pub fn get_children(&mut self, path: impl AsRef<Path>) -> Option<Vec<ImfsEntry>> {
        self.inner.children(path)?
            .into_iter()
            .map(|path| path.to_path_buf())
            .collect::<Vec<PathBuf>>()
            .into_iter()
            .map(|path| self.get(path))
            .collect()
    }
}

/// A reference to file or folder in an `Imfs`. Can only be produced by the
/// entry existing in the Imfs, but can later point to nothing if something
/// would invalidate that path.
///
/// This struct does not borrow from the Imfs since every operation has the
/// possibility to mutate the underlying data structure and move memory around.
pub struct ImfsEntry {
    path: PathBuf,
    is_file: bool,
}

impl ImfsEntry {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn contents<'imfs>(
        &self,
        imfs: &'imfs mut Imfs<impl ImfsFetcher>,
    ) -> Option<&'imfs [u8]> {
        imfs.get_contents(&self.path)
    }

    pub fn children(
        &self,
        imfs: &mut Imfs<impl ImfsFetcher>,
    ) -> Option<Vec<ImfsEntry>> {
        imfs.get_children(&self.path)
    }

    pub fn is_file(&self) -> bool {
        self.is_file
    }

    pub fn is_directory(&self) -> bool {
        !self.is_file
    }
}

/// Internal structure describing potentially partially-resident files and
/// folders in the `Imfs`.
pub enum ImfsItem {
    File(ImfsFile),
    Directory(ImfsDirectory),
}

impl ImfsItem {
    fn path(&self) -> &Path {
        match self {
            ImfsItem::File(file) => &file.path,
            ImfsItem::Directory(dir) => &dir.path,
        }
    }
}

pub struct ImfsFile {
    pub(super) path: PathBuf,
    pub(super) contents: Option<Vec<u8>>,
}

pub struct ImfsDirectory {
    pub(super) path: PathBuf,
    pub(super) children_enumerated: bool,
}