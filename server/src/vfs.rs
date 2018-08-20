use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    fs,
    io,
};

pub struct Vfs {
    contents: HashMap<PathBuf, Vec<u8>>,
    items: HashMap<PathBuf, VfsItem>,
    roots: HashSet<PathBuf>,
}

impl Vfs {
    pub fn new() -> Vfs {
        Vfs {
            contents: HashMap::new(),
            items: HashMap::new(),
            roots: HashSet::new(),
        }
    }

    pub fn add_root<'a, 'b>(&'a mut self, root_path: &'b Path) -> io::Result<&'a VfsItem> {
        debug_assert!(root_path.is_absolute());

        self.roots.insert(root_path.to_path_buf());

        VfsItem::get(self, root_path)
    }

    pub fn get_roots(&self) -> &HashSet<PathBuf> {
        &self.roots
    }

    pub fn get(&mut self, path: &Path) -> Option<&VfsItem> {
        debug_assert!(path.is_absolute());
        debug_assert!(self.is_valid_path(path));

        self.items.get(path)
    }

    pub fn remove(&mut self, path: &Path) {
        debug_assert!(path.is_absolute());
        debug_assert!(self.is_valid_path(path));

        match self.items.remove(path) {
            Some(item) => match item {
                VfsItem::File(_) => {
                    self.contents.remove(path);
                },
                VfsItem::Directory(VfsDirectory { children, .. }) => {
                    for child_path in &children {
                        self.remove(child_path);
                    }
                },
            },
            None => {},
        }
    }

    pub fn add_or_update<'a, 'b>(&'a mut self, path: &'b Path) -> io::Result<&'a VfsItem> {
        debug_assert!(path.is_absolute());
        debug_assert!(self.is_valid_path(path));

        VfsItem::get(self, path)
    }

    fn is_valid_path(&self, path: &Path) -> bool {
        let mut is_valid_path = false;

        for root_path in &self.roots {
            if path.starts_with(root_path) {
                is_valid_path = true;
                break;
            }
        }

        is_valid_path
    }
}

pub struct VfsFile {
    path: PathBuf,
}

impl VfsFile {
    pub fn read_contents<'file, 'vfs>(&'file self, vfs: &'vfs mut Vfs) -> io::Result<&'vfs [u8]> {
        if !vfs.contents.contains_key(&self.path) {
            let contents = fs::read(&self.path)?;
            vfs.contents.insert(self.path.clone(), contents);
        }

        Ok(vfs.contents.get(&self.path).unwrap())
    }
}

pub struct VfsDirectory {
    path: PathBuf,
    children: HashSet<PathBuf>,
}

pub enum VfsItem {
    File(VfsFile),
    Directory(VfsDirectory),
}

impl VfsItem {
    fn get<'a, 'b>(vfs: &'a mut Vfs, root_path: &'b Path) -> io::Result<&'a VfsItem> {
        let metadata = fs::metadata(root_path)?;

        if metadata.is_file() {
            let item = VfsItem::File(VfsFile {
                path: root_path.to_path_buf(),
            });

            vfs.items.insert(root_path.to_path_buf(), item);

            let contents = fs::read(root_path)?;
            vfs.contents.insert(root_path.to_path_buf(), contents);

            Ok(vfs.items.get(root_path).unwrap())
        } else if metadata.is_dir() {
            let mut children = HashSet::new();

            for entry in fs::read_dir(root_path)? {
                let entry = entry?;
                let path = entry.path();

                VfsItem::get(vfs, &path)?;

                children.insert(path);
            }

            let item = VfsItem::Directory(VfsDirectory {
                path: root_path.to_path_buf(),
                children,
            });

            vfs.items.insert(root_path.to_path_buf(), item);

            Ok(vfs.items.get(root_path).unwrap())
        } else {
            unimplemented!();
        }
    }
}