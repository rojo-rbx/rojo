use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
}
