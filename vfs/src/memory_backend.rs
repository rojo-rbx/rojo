use std::collections::{BTreeSet, HashMap};
use std::io;
use std::path::{Path, PathBuf};

use crate::{Metadata, ReadDir, VfsBackend, VfsEvent, VfsSnapshot};

/// `VfsBackend` that reads from an in-memory filesystem.
#[derive(Debug)]
#[non_exhaustive]
pub struct MemoryBackend {
    entries: HashMap<PathBuf, Entry>,
    orphans: BTreeSet<PathBuf>,
}

impl MemoryBackend {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            orphans: BTreeSet::new(),
        }
    }

    pub fn load_snapshot<P: Into<PathBuf>>(&mut self, path: P, snapshot: VfsSnapshot) {
        let path = path.into();

        if let Some(parent_path) = path.parent() {
            if let Some(parent_entry) = self.entries.get_mut(parent_path) {
                if let Entry::Dir { children } = parent_entry {
                    children.insert(path.clone());
                } else {
                    panic!(
                        "Tried to load snapshot as child of file, {}",
                        parent_path.display()
                    );
                }
            } else {
                self.orphans.insert(path.clone());
            }
        } else {
            self.orphans.insert(path.clone());
        }

        match snapshot {
            VfsSnapshot::File { contents } => {
                self.entries.insert(path, Entry::File { contents });
            }
            VfsSnapshot::Dir { children } => {
                self.entries.insert(
                    path.clone(),
                    Entry::Dir {
                        children: BTreeSet::new(),
                    },
                );

                for (child_name, child) in children {
                    let full_path = path.join(child_name);
                    self.load_snapshot(full_path, child);
                }
            }
        };
    }
}

#[derive(Debug)]
enum Entry {
    File { contents: Vec<u8> },

    Dir { children: BTreeSet<PathBuf> },
}

impl VfsBackend for MemoryBackend {
    fn read(&mut self, _path: &Path) -> io::Result<Vec<u8>> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "MemoryBackend doesn't do anything",
        ))
    }

    fn write(&mut self, _path: &Path, _data: &[u8]) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "MemoryBackend doesn't do anything",
        ))
    }

    fn read_dir(&mut self, _path: &Path) -> io::Result<ReadDir> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "MemoryBackend doesn't do anything",
        ))
    }

    fn remove_file(&mut self, _path: &Path) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "MemoryBackend doesn't do anything",
        ))
    }

    fn remove_dir_all(&mut self, _path: &Path) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "MemoryBackend doesn't do anything",
        ))
    }

    fn metadata(&mut self, _path: &Path) -> io::Result<Metadata> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "MemoryBackend doesn't do anything",
        ))
    }

    fn event_receiver(&self) -> crossbeam_channel::Receiver<VfsEvent> {
        crossbeam_channel::never()
    }

    fn watch(&mut self, _path: &Path) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "MemoryBackend doesn't do anything",
        ))
    }

    fn unwatch(&mut self, _path: &Path) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "MemoryBackend doesn't do anything",
        ))
    }
}
