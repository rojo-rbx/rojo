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

        self.read_from_disk(path)
    }

    pub fn path_created(&mut self, path: &Path) -> io::Result<()> {
        debug_assert!(path.is_absolute());
        debug_assert!(self.is_within_roots(path));

        self.read_from_disk(path)
    }

    pub fn path_updated(&mut self, path: &Path) -> io::Result<()> {
        debug_assert!(path.is_absolute());
        debug_assert!(self.is_within_roots(path));

        if let Some(parent_path) = path.parent() {
            if self.is_within_roots(parent_path) && self.get(parent_path).is_none() {
                self.path_created(parent_path)?;
            }
        } else {
            self.read_from_disk(path)?;
        }

        Ok(())
    }

    pub fn path_removed(&mut self, path: &Path) -> io::Result<()> {
        debug_assert!(path.is_absolute());
        debug_assert!(self.is_within_roots(path));

        if let Some(parent_path) = path.parent() {
            self.unlink_child(parent_path, path);
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

    fn unlink_child(&mut self, parent: &Path, child: &Path) {
        let parent_item = self.items.get_mut(parent);

        match parent_item {
            Some(ImfsItem::Directory(directory)) => {
                directory.children.remove(child);
            },
            _ => {
                panic!("Tried to unlink child of path that wasn't a directory!");
            },
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

    fn is_within_roots(&self, path: &Path) -> bool {
        let kind = self.classify_path(path);

        kind == PathKind::Root || kind == PathKind::InRoot
    }

    fn classify_path(&self, path: &Path) -> PathKind {
        for root_path in &self.roots {
            if root_path == path {
                return PathKind::Root;
            }

            if path.starts_with(root_path) {
                return PathKind::InRoot;
            }
        }

        PathKind::NotInRoot
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum PathKind {
    Root,
    InRoot,
    NotInRoot,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ImfsFile {
    pub path: PathBuf,
    pub contents: Vec<u8>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ImfsDirectory {
    pub path: PathBuf,
    pub children: HashSet<PathBuf>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum ImfsItem {
    File(ImfsFile),
    Directory(ImfsDirectory),
}