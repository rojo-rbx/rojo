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

    fn read_if_not_exists(&mut self, path: &Path) {
        if !self.inner.contains_key(path) {
            let item = self.fetcher.read_item(path);
            self.inner.insert(path.to_path_buf(), item);
        }
    }

    pub fn get(&mut self, path: impl AsRef<Path>) -> Option<ImfsEntry> {
        self.read_if_not_exists(path.as_ref());
        let item = self.inner.get(path.as_ref())?;
        Some(ImfsEntry {
            path: item.path().to_path_buf(),
        })
    }

    pub fn get_contents(&mut self, path: impl AsRef<Path>) -> Option<&[u8]> {
        self.read_if_not_exists(path.as_ref());

        match self.inner.get_mut(path.as_ref())? {
            ImfsItem::File(file) => {
                if file.contents.is_none() {
                    file.contents = Some(self.fetcher.read_contents(path));
                }

                Some(file.contents.as_ref().unwrap())
            }
            ImfsItem::Directory(_) => None
        }
    }

    pub fn get_children(&mut self, path: impl AsRef<Path>) -> Option<Vec<ImfsEntry>> {
        Some(self.inner.children(path)?
            .into_iter()
            .map(|path| ImfsEntry {
                path: path.to_path_buf()
            })
            .collect())
    }
}

pub struct ImfsEntry {
    path: PathBuf,
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
}

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
    path: PathBuf,
    contents: Option<Vec<u8>>,
}

pub struct ImfsDirectory {
    path: PathBuf,
    children_enumerated: bool,
}