use std::{
    collections::{HashMap, HashSet},
    path::{self, Path, PathBuf},
    fs,
    io,
};

use serde_derive::{Serialize, Deserialize};

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

/// The in-memory filesystem keeps a mirror of all files being watched by Rojo
/// in order to deduplicate file changes in the case of bidirectional syncing
/// from Roblox Studio.
///
/// It also enables Rojo to quickly generate React-like snapshots to make
/// reasoning about instances and how they relate to files easier.
#[derive(Debug, Clone)]
pub struct Imfs {
    items: HashMap<PathBuf, ImfsItem>,
    roots: HashSet<PathBuf>,
}

impl Imfs {
    pub fn new() -> Imfs {
        Imfs {
            items: HashMap::new(),
            roots: HashSet::new(),
        }
    }

    pub fn add_roots_from_project(&mut self, project: &Project) -> io::Result<()> {
        add_sync_points(self, &project.tree)
    }

    pub fn get_roots(&self) -> &HashSet<PathBuf> {
        &self.roots
    }

    pub fn get_items(&self) -> &HashMap<PathBuf, ImfsItem> {
        &self.items
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

        self.descend_and_read_from_disk(path)
    }

    pub fn path_created(&mut self, path: &Path) -> io::Result<()> {
        debug_assert!(path.is_absolute());
        debug_assert!(self.is_within_roots(path));

        self.descend_and_read_from_disk(path)
    }

    pub fn path_updated(&mut self, path: &Path) -> io::Result<()> {
        debug_assert!(path.is_absolute());
        debug_assert!(self.is_within_roots(path));

        self.descend_and_read_from_disk(path)
    }

    pub fn path_removed(&mut self, path: &Path) -> io::Result<()> {
        debug_assert!(path.is_absolute());
        debug_assert!(self.is_within_roots(path));

        self.remove_item(path);

        if let Some(parent_path) = path.parent() {
            self.unlink_child(parent_path, path);
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

    pub fn get_root_for_path<'a>(&'a self, path: &Path) -> Option<&'a Path> {
        for root_path in &self.roots {
            if path.starts_with(root_path) {
                return Some(root_path);
            }
        }

        None
    }

    fn remove_item(&mut self, path: &Path) {
        if let Some(ImfsItem::Directory(directory)) = self.items.remove(path) {
            for child_path in &directory.children {
                self.remove_item(child_path);
            }
        }
    }

    fn unlink_child(&mut self, parent: &Path, child: &Path) {
        let parent_item = self.items.get_mut(parent);

        match parent_item {
            Some(ImfsItem::Directory(directory)) => {
                directory.children.remove(child);
            },
            _ => {},
        }
    }

    fn link_child(&mut self, parent: &Path, child: &Path) {
        if self.is_within_roots(parent) {
            let parent_item = self.items.get_mut(parent);

            match parent_item {
                Some(ImfsItem::Directory(directory)) => {
                    directory.children.insert(child.to_path_buf());
                },
                _ => {
                    panic!("Tried to link child of path that wasn't a directory!");
                },
            }
        }
    }

    fn descend_and_read_from_disk(&mut self, path: &Path) -> io::Result<()> {
        let root_path = self.get_root_path(path)
            .expect("Tried to mkdirp for path that wasn't within roots!");

        // If this path is a root, we should read the entire thing.
        if root_path == path {
            self.read_from_disk(path)?;
            return Ok(());
        }

        let relative_path = path.strip_prefix(root_path).unwrap();
        let mut current_path = root_path.to_path_buf();

        for component in relative_path.components() {
            match component {
                path::Component::Normal(name) => {
                    let next_path = current_path.join(name);

                    if self.items.contains_key(&next_path) {
                        current_path = next_path;
                    } else {
                        self.read_from_disk(&current_path)?;
                        break;
                    }
                },
                _ => unreachable!(),
            }
        }

        Ok(())
    }

    fn read_from_disk(&mut self, path: &Path) -> io::Result<()> {
        let metadata = fs::metadata(path)?;

        if metadata.is_file() {
            let contents = fs::read(path)?;
            let item = ImfsItem::File(ImfsFile {
                path: path.to_path_buf(),
                contents,
            });

            self.items.insert(path.to_path_buf(), item);

            if let Some(parent_path) = path.parent() {
                self.link_child(parent_path, path);
            }

            Ok(())
        } else if metadata.is_dir() {
            let item = ImfsItem::Directory(ImfsDirectory {
                path: path.to_path_buf(),
                children: HashSet::new(),
            });

            self.items.insert(path.to_path_buf(), item);

            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let child_path = entry.path();

                self.read_from_disk(&child_path)?;
            }

            if let Some(parent_path) = path.parent() {
                self.link_child(parent_path, path);
            }

            Ok(())
        } else {
            panic!("Unexpected non-file, non-directory item");
        }
    }

    fn get_root_path<'a>(&'a self, path: &Path) -> Option<&'a Path> {
        for root_path in &self.roots {
            if path.starts_with(root_path) {
                return Some(root_path)
            }
        }

        None
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImfsFile {
    pub path: PathBuf,
    pub contents: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImfsDirectory {
    pub path: PathBuf,
    pub children: HashSet<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ImfsItem {
    File(ImfsFile),
    Directory(ImfsDirectory),
}