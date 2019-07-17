use std::path::{Path, PathBuf};

use crate::path_map::PathMap;

pub trait ImfsFetcher {
    fn read_item(&self, path: impl AsRef<Path>) -> ImfsItem;
    fn read_children(&self, path: impl AsRef<Path>) -> Vec<ImfsItem>;
    fn read_contents(&self, path: impl AsRef<Path>) -> Vec<u8>;
}

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

    pub fn get(&mut self, path: impl AsRef<Path>) -> Option<ImfsEntry> {
        let path = path.as_ref();

        if self.inner.contains_key(path) {
            return Some(ImfsEntry {
                path: path.to_path_buf(),
            });
        }

        // TODO: If this path is a child of any of our existing root paths, read
        // down the path to this path so that this one isn't orphaned.

        let item = self.fetcher.read_item(path);
        self.inner.insert(path.to_path_buf(), item);

        unimplemented!("AHH?");
    }

    pub fn get_contents(&mut self, path: impl AsRef<Path>) -> Option<&[u8]> {
        let item = self.inner.get(path.as_ref())?;
        match item {
            ImfsItem::File(file) => file.contents.as_ref().map(Vec::as_slice),
            ImfsItem::Directory(_) => None,
        }
    }
}

pub struct ImfsEntry {
    path: PathBuf,
}

impl ImfsEntry {
    pub fn contents<'imfs>(
        &self,
        imfs: &'imfs mut Imfs<impl ImfsFetcher>,
    ) -> Option<&'imfs [u8]> {
        let item = imfs.inner.get(&self.path)?;
        match item {
            ImfsItem::File(file) => file.contents.as_ref().map(Vec::as_slice),
            ImfsItem::Directory(_) => None,
        }
    }

    pub fn children<'imfs>(
        &self,
        imfs: &'imfs mut Imfs<impl ImfsFetcher>,
    ) -> Option<&'imfs [u8]> {
        let item = imfs.inner.get(&self.path)?;
        match item {
            ImfsItem::File(file) => file.contents.as_ref().map(Vec::as_slice),
            ImfsItem::Directory(_) => None,
        }
    }
}

pub enum ImfsItem {
    File(ImfsFile),
    Directory(ImfsDirectory),
}

pub struct ImfsFile {
    path: PathBuf,
    contents: Option<Vec<u8>>,
}

pub struct ImfsDirectory {
    path: PathBuf,
    children_enumerated: bool,
}