use std::path::{Path, PathBuf};

use crate::path_map::PathMap;

pub trait ImfsFetcher {}

pub struct Imfs<F> {
    inner: PathMap<ImfsItem>,
    fetcher: F,
}

impl<F: ImfsFetcher> Imfs<F> {
    pub fn get(&self, path: impl AsRef<Path>) -> Option<&ImfsItem> {
        if let Some(entry) = self.inner.get(path) {
            return Some(entry);
        }

        unimplemented!("AHH?");
    }
}

pub enum ImfsItem {
    File(ImfsFile),
    Directory(ImfsDirectory),
}

pub struct ImfsFile {
    pub path: PathBuf,
    pub contents: Vec<u8>,
}

pub struct ImfsDirectory {
    pub path: PathBuf,
}