// This file is non-critical and used for testing, so it's okay if it's unused.
#![allow(unused)]

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum VfsSnapshot {
    File(FileSnapshot),
    Directory(DirectorySnapshot),
}

impl VfsSnapshot {
    /// Create a new file VfsSnapshot with the given contents.
    pub fn file(contents: impl Into<Vec<u8>>) -> VfsSnapshot {
        VfsSnapshot::File(FileSnapshot {
            contents: contents.into(),
        })
    }

    /// Create a new directory VfsSnapshot with the given children.
    pub fn dir<S: Into<String>>(children: HashMap<S, VfsSnapshot>) -> VfsSnapshot {
        let children = children.into_iter().map(|(k, v)| (k.into(), v)).collect();

        VfsSnapshot::Directory(DirectorySnapshot { children })
    }

    pub fn empty_dir() -> VfsSnapshot {
        VfsSnapshot::Directory(DirectorySnapshot {
            children: Default::default(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct FileSnapshot {
    pub contents: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct DirectorySnapshot {
    pub children: HashMap<String, VfsSnapshot>,
}
