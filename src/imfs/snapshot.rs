// This file is non-critical and used for testing, so it's okay if it's unused.
#![allow(unused)]

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum ImfsSnapshot {
    File(FileSnapshot),
    Directory(DirectorySnapshot),
}

impl ImfsSnapshot {
    /// Create a new file ImfsSnapshot with the given contents.
    pub fn file(contents: impl Into<Vec<u8>>) -> ImfsSnapshot {
        ImfsSnapshot::File(FileSnapshot {
            contents: contents.into(),
        })
    }

    /// Create a new directory ImfsSnapshot with the given children.
    pub fn dir<S: Into<String>>(children: HashMap<S, ImfsSnapshot>) -> ImfsSnapshot {
        let children = children.into_iter().map(|(k, v)| (k.into(), v)).collect();

        ImfsSnapshot::Directory(DirectorySnapshot { children })
    }

    pub fn empty_dir() -> ImfsSnapshot {
        ImfsSnapshot::Directory(DirectorySnapshot {
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
    pub children: HashMap<String, ImfsSnapshot>,
}
