use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;
use std::fs;
use std::fs::DirEntry;
use std::path::PathBuf;

/// A slice of a tree of files. Can be loaded into an
/// [`InMemoryFs`](struct.InMemoryFs.html).
#[derive(Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub enum VfsSnapshot {
    File {
        contents: Vec<u8>,
    },

    Dir {
        children: BTreeMap<String, VfsSnapshot>,
    },
}

impl VfsSnapshot {
    pub fn file<C: Into<Vec<u8>>>(contents: C) -> Self {
        Self::File {
            contents: contents.into(),
        }
    }

    pub fn dir<K: Into<String>, I: IntoIterator<Item = (K, VfsSnapshot)>>(children: I) -> Self {
        Self::Dir {
            children: children
                .into_iter()
                .map(|(key, value)| (key.into(), value))
                .collect(),
        }
    }

    pub fn empty_file() -> Self {
        Self::File {
            contents: Vec::new(),
        }
    }

    pub fn empty_dir() -> Self {
        Self::Dir {
            children: BTreeMap::new(),
        }
    }

    pub fn from_fs_path(path: &PathBuf) -> Result<Self, PathBuf> {
        if path.is_file() {
            fs::read_to_string(path).ok()
                .map(|content| Self::file(content))
                .ok_or(path.to_owned())
        } else {
            let entries: Result<Vec<DirEntry>, PathBuf> = fs::read_dir(path)
                .map_err(|_| path.to_owned())?
                .map(|entry| entry.map_err(|_| path.to_owned()))
                .into_iter()
                .collect();

            let vfs_entries: Result<Vec<(String, Self)>, PathBuf> = entries?.iter()
                .map(|entry| {
                    let path = entry.path();

                    path.file_name()
                        .and_then(|file_name| file_name.to_str())
                        .ok_or(path.to_owned())
                        .and_then(|file_name| {
                            Self::from_fs_path(&path)
                                .map(|snapshot| (file_name.to_owned(), snapshot))
                        })
                }).into_iter().collect();

            Ok(Self::dir(vfs_entries?))
        }
    }
}
