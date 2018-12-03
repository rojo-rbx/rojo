use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    fs,
    io,
};

use crate::project::{Project, ProjectNode};

fn add_sync_points(imfs: &mut Imfs, project_node: &ProjectNode) -> io::Result<()> {
    match project_node {
        ProjectNode::Instance(node) => {
            for child in node.children.values() {
                add_sync_points(imfs, child)?;
            }
        },
        ProjectNode::SyncPoint(node) => {
            imfs.add_root(&node.path)?;
        },
    }

    Ok(())
}

/// The in-memory filesystem keeps a mirror of all files being watcher by Rojo
/// in order to deduplicate file changes in the case of bidirectional syncing
/// from Roblox Studio.
#[derive(Debug)]
pub struct Imfs {
    items: HashMap<PathBuf, ImfsItem>,
    roots: HashSet<PathBuf>,
}

impl Imfs {
    pub fn new(project: &Project) -> io::Result<Imfs> {
        let mut imfs = Imfs::empty();

        add_sync_points(&mut imfs, &project.tree)?;

        Ok(imfs)
    }

    pub fn empty() -> Imfs {
        Imfs {
            items: HashMap::new(),
            roots: HashSet::new(),
        }
    }

    pub fn get_roots(&self) -> &HashSet<PathBuf> {
        &self.roots
    }

    pub fn get(&self, path: &Path) -> Option<&ImfsItem> {
        debug_assert!(path.is_absolute());
        debug_assert!(self.is_within_roots(path));

        self.items.get(path)
    }

    pub fn add_root(&mut self, path: &Path) -> io::Result<()> {
        debug_assert!(path.is_absolute());
        debug_assert!(!self.is_within_roots(path));

        self.roots.insert(path.to_path_buf());

        ImfsItem::read_from_disk(self, path)?;
        Ok(())
    }

    pub fn path_created(&mut self, path: &Path) -> io::Result<()> {
        debug_assert!(path.is_absolute());
        debug_assert!(self.is_within_roots(path));

        if let Some(parent_path) = path.parent() {
            if self.is_within_roots(parent_path) && self.get(parent_path).is_none() {
                self.path_created(parent_path)?;
            }
        }

        ImfsItem::read_from_disk(self, path)?;
        Ok(())
    }

    pub fn path_updated(&mut self, path: &Path) -> io::Result<()> {
        debug_assert!(path.is_absolute());
        debug_assert!(self.is_within_roots(path));

        if let Some(parent_path) = path.parent() {
            if self.is_within_roots(parent_path) && self.get(parent_path).is_none() {
                self.path_created(parent_path)?;
            }
        }

        ImfsItem::read_from_disk(self, path)?;
        Ok(())
    }

    pub fn path_removed(&mut self, path: &Path) -> io::Result<()> {
        debug_assert!(path.is_absolute());
        debug_assert!(self.is_within_roots(path));

        if let Some(parent_path) = path.parent() {
            if self.is_within_roots(parent_path) {
                if let Some(ImfsItem::Directory(parent)) = self.items.get_mut(parent_path) {
                    parent.children.remove(path);
                }
            }
        }

        if let Some(ImfsItem::Directory(directory)) = self.items.remove(path) {
            for child_path in &directory.children {
                self.path_removed(child_path)?;
            }
        }

        Ok(())
    }

    pub fn path_moved(&mut self, from_path: &Path, to_path: &Path) -> io::Result<()> {
        debug_assert!(from_path.is_absolute());
        debug_assert!(self.is_within_roots(from_path));
        debug_assert!(to_path.is_absolute());
        debug_assert!(self.is_within_roots(to_path));

        self.path_removed(from_path)?;
        self.path_created(to_path)?;
        Ok(())
    }

    fn is_within_roots(&self, path: &Path) -> bool {
        for root_path in &self.roots {
            if path.starts_with(root_path) {
                return true;
            }
        }

        false
    }
}

#[derive(Debug)]
pub struct ImfsFile {
    pub path: PathBuf,
    pub contents: Vec<u8>,
}

#[derive(Debug)]
pub struct ImfsDirectory {
    pub path: PathBuf,
    pub children: HashSet<PathBuf>,
}

#[derive(Debug)]
pub enum ImfsItem {
    File(ImfsFile),
    Directory(ImfsDirectory),
}

impl ImfsItem {
    fn read_from_disk<'a, 'b>(vfs: &'a mut Imfs, path: &'b Path) -> io::Result<&'a ImfsItem> {
        let metadata = fs::metadata(path)?;

        if metadata.is_file() {
            let contents = fs::read(path)?;
            let item = ImfsItem::File(ImfsFile {
                path: path.to_path_buf(),
                contents,
            });

            vfs.items.insert(path.to_path_buf(), item);

            Ok(&vfs.items[path])
        } else if metadata.is_dir() {
            let mut children = HashSet::new();

            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let child_path = entry.path();

                ImfsItem::read_from_disk(vfs, &child_path)?;

                children.insert(child_path);
            }

            let item = ImfsItem::Directory(ImfsDirectory {
                path: path.to_path_buf(),
                children,
            });

            vfs.items.insert(path.to_path_buf(), item);

            Ok(&vfs.items[path])
        } else {
            panic!("Unexpected non-file, non-directory item");
        }
    }
}