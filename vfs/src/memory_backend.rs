use std::collections::{BTreeSet, HashMap, VecDeque};
use std::io;
use std::path::{Path, PathBuf};

use crate::{DirEntry, Metadata, ReadDir, VfsBackend, VfsEvent, VfsSnapshot};

/// `VfsBackend` that reads from an in-memory filesystem, intended for setting
/// up testing scenarios quickly.
#[derive(Debug)]
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

    pub fn load_snapshot<P: Into<PathBuf>>(
        &mut self,
        path: P,
        snapshot: VfsSnapshot,
    ) -> io::Result<()> {
        let path = path.into();

        if let Some(parent_path) = path.parent() {
            if let Some(parent_entry) = self.entries.get_mut(parent_path) {
                if let Entry::Dir { children } = parent_entry {
                    children.insert(path.clone());
                } else {
                    return must_be_dir(parent_path);
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
                    self.load_snapshot(full_path, child)?;
                }
            }
        }

        Ok(())
    }

    fn remove(&mut self, root_path: PathBuf) {
        self.orphans.remove(&root_path);

        let mut to_remove = VecDeque::new();
        to_remove.push_back(root_path);

        while let Some(path) = to_remove.pop_front() {
            if let Some(Entry::Dir { children }) = self.entries.remove(&path) {
                to_remove.extend(children);
            }
        }
    }
}

#[derive(Debug)]
enum Entry {
    File { contents: Vec<u8> },

    Dir { children: BTreeSet<PathBuf> },
}

impl VfsBackend for MemoryBackend {
    fn read(&mut self, path: &Path) -> io::Result<Vec<u8>> {
        match self.entries.get(path) {
            Some(Entry::File { contents }) => Ok(contents.clone()),
            Some(Entry::Dir { .. }) => must_be_file(path),
            None => not_found(path),
        }
    }

    fn write(&mut self, path: &Path, data: &[u8]) -> io::Result<()> {
        self.load_snapshot(
            path,
            VfsSnapshot::File {
                contents: data.to_owned(),
            },
        )
    }

    fn read_dir(&mut self, path: &Path) -> io::Result<ReadDir> {
        match self.entries.get(path) {
            Some(Entry::Dir { children }) => {
                let iter = children
                    .clone()
                    .into_iter()
                    .map(|path| Ok(DirEntry { path }));

                Ok(ReadDir {
                    inner: Box::new(iter),
                })
            }
            Some(Entry::File { .. }) => must_be_dir(path),
            None => not_found(path),
        }
    }

    fn remove_file(&mut self, path: &Path) -> io::Result<()> {
        match self.entries.get(path) {
            Some(Entry::File { .. }) => {
                self.remove(path.to_owned());
                Ok(())
            }
            Some(Entry::Dir { .. }) => must_be_file(path),
            None => not_found(path),
        }
    }

    fn remove_dir_all(&mut self, path: &Path) -> io::Result<()> {
        match self.entries.get(path) {
            Some(Entry::Dir { .. }) => {
                self.remove(path.to_owned());
                Ok(())
            }
            Some(Entry::File { .. }) => must_be_dir(path),
            None => not_found(path),
        }
    }

    fn metadata(&mut self, path: &Path) -> io::Result<Metadata> {
        match self.entries.get(path) {
            Some(Entry::File { .. }) => Ok(Metadata { is_file: true }),
            Some(Entry::Dir { .. }) => Ok(Metadata { is_file: false }),
            None => not_found(path),
        }
    }

    fn event_receiver(&self) -> crossbeam_channel::Receiver<VfsEvent> {
        crossbeam_channel::never()
    }

    fn watch(&mut self, _path: &Path) -> io::Result<()> {
        Ok(())
    }

    fn unwatch(&mut self, _path: &Path) -> io::Result<()> {
        Ok(())
    }
}

fn must_be_file<T>(path: &Path) -> io::Result<T> {
    Err(io::Error::new(
        io::ErrorKind::Other,
        format!(
            "path {} was a directory, but must be a file",
            path.display()
        ),
    ))
}

fn must_be_dir<T>(path: &Path) -> io::Result<T> {
    Err(io::Error::new(
        io::ErrorKind::Other,
        format!(
            "path {} was a file, but must be a directory",
            path.display()
        ),
    ))
}

fn not_found<T>(path: &Path) -> io::Result<T> {
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("path {} not found", path.display()),
    ))
}
