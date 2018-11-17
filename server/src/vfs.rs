use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    fs,
    io,
};

#[derive(Debug)]
pub struct Vfs {
    items: HashMap<PathBuf, VfsItem>,
    roots: HashSet<PathBuf>,
}

impl Vfs {
    pub fn new() -> Vfs {
        Vfs {
            items: HashMap::new(),
            roots: HashSet::new(),
        }
    }

    pub fn get_roots(&self) -> &HashSet<PathBuf> {
        &self.roots
    }

    pub fn get(&self, path: &Path) -> Option<&VfsItem> {
        debug_assert!(path.is_absolute());
        debug_assert!(self.is_within_roots(path));

        self.items.get(path)
    }

    pub fn add_root(&mut self, path: &Path) -> io::Result<()> {
        debug_assert!(path.is_absolute());
        debug_assert!(!self.is_within_roots(path));

        self.roots.insert(path.to_path_buf());

        VfsItem::read_from_disk(self, path)?;
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

        VfsItem::read_from_disk(self, path)?;
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

        VfsItem::read_from_disk(self, path)?;
        Ok(())
    }

    pub fn path_removed(&mut self, path: &Path) -> io::Result<()> {
        debug_assert!(path.is_absolute());
        debug_assert!(self.is_within_roots(path));

        if let Some(parent_path) = path.parent() {
            if self.is_within_roots(parent_path) {
                if let Some(VfsItem::Directory(parent)) = self.items.get_mut(parent_path) {
                    parent.children.remove(path);
                }
            }
        }

        match self.items.remove(path) {
            Some(VfsItem::Directory(directory)) => {
                for child_path in &directory.children {
                    self.path_removed(child_path)?;
                }
            },
            _ => {},
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
pub struct VfsFile {
    pub path: PathBuf,
    pub contents: Vec<u8>,
}

#[derive(Debug)]
pub struct VfsDirectory {
    pub path: PathBuf,
    pub children: HashSet<PathBuf>,
}

#[derive(Debug)]
pub enum VfsItem {
    File(VfsFile),
    Directory(VfsDirectory),
}

impl VfsItem {
    fn read_from_disk<'a, 'b>(vfs: &'a mut Vfs, path: &'b Path) -> io::Result<&'a VfsItem> {
        let metadata = fs::metadata(path)?;

        if metadata.is_file() {
            let contents = fs::read(path)?;
            let item = VfsItem::File(VfsFile {
                path: path.to_path_buf(),
                contents,
            });

            vfs.items.insert(path.to_path_buf(), item);

            Ok(vfs.items.get(path).unwrap())
        } else if metadata.is_dir() {
            let mut children = HashSet::new();

            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let child_path = entry.path();

                VfsItem::read_from_disk(vfs, &child_path)?;

                children.insert(child_path);
            }

            let item = VfsItem::Directory(VfsDirectory {
                path: path.to_path_buf(),
                children,
            });

            vfs.items.insert(path.to_path_buf(), item);

            Ok(vfs.items.get(path).unwrap())
        } else {
            panic!("Unexpected non-file, non-directory item");
        }
    }
}