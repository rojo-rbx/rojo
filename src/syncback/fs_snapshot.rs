use std::{
    collections::{HashMap, HashSet},
    fmt, io,
    path::{Path, PathBuf},
};

use memofs::Vfs;

/// A simple representation of a subsection of a file system.
#[derive(Default)]
pub struct FsSnapshot {
    /// Paths representing new files mapped to their contents.
    added_files: HashMap<PathBuf, Vec<u8>>,
    /// Paths representing new directories.
    added_dirs: HashSet<PathBuf>,
    /// Paths representing removed files.
    removed_files: HashSet<PathBuf>,
    /// Paths representing removed directories.
    removed_dirs: HashSet<PathBuf>,
}

impl FsSnapshot {
    /// Creates a new `FsSnapshot`.
    pub fn new() -> Self {
        Self {
            added_files: HashMap::new(),
            added_dirs: HashSet::new(),
            removed_files: HashSet::new(),
            removed_dirs: HashSet::new(),
        }
    }

    /// Adds the given path to the `FsSnapshot` as a file with the given
    /// contents, then returns it.
    pub fn with_added_file<P: AsRef<Path>>(mut self, path: P, data: Vec<u8>) -> Self {
        self.added_files.insert(path.as_ref().to_path_buf(), data);
        self
    }

    /// Adds the given path to the `FsSnapshot` as a file with the given
    /// then returns it.
    pub fn with_added_dir<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.added_dirs.insert(path.as_ref().to_path_buf());
        self
    }

    /// Merges two `FsSnapshot`s together.
    #[inline]
    pub fn merge(&mut self, other: Self) {
        self.added_files.extend(other.added_files);
        self.added_dirs.extend(other.added_dirs);
        self.removed_files.extend(other.removed_files);
        self.removed_dirs.extend(other.removed_dirs);
    }

    /// Adds the provided path as a file with the given contents.
    pub fn add_file<P: AsRef<Path>>(&mut self, path: P, data: Vec<u8>) {
        self.added_files.insert(path.as_ref().to_path_buf(), data);
    }

    /// Adds the provided path as a directory.
    pub fn add_dir<P: AsRef<Path>>(&mut self, path: P) {
        self.added_dirs.insert(path.as_ref().to_path_buf());
    }

    /// Removes the provided path, as a file.
    pub fn remove_file<P: AsRef<Path>>(&mut self, path: P) {
        self.removed_files.insert(path.as_ref().to_path_buf());
    }

    /// Removes the provided path, as a directory.
    pub fn remove_dir<P: AsRef<Path>>(&mut self, path: P) {
        self.removed_dirs.insert(path.as_ref().to_path_buf());
    }

    /// Writes the `FsSnapshot` to the provided VFS, using the provided `base`
    /// as a root for the other paths in the `FsSnapshot`.
    ///
    /// This includes removals, but makes no effort to minimize work done.
    pub fn write_to_vfs<P: AsRef<Path>>(&self, base: P, vfs: &Vfs) -> io::Result<()> {
        let base_path = base.as_ref();
        for dir_path in &self.added_dirs {
            match vfs.create_dir_all(base_path.join(dir_path)) {
                Ok(_) => (),
                Err(err) if err.kind() == io::ErrorKind::AlreadyExists => (),
                Err(err) => return Err(err),
            };
        }
        for (path, contents) in &self.added_files {
            vfs.write(base_path.join(path), contents)?;
        }
        for dir_path in &self.removed_dirs {
            vfs.remove_dir_all(base_path.join(dir_path))?;
        }
        for path in &self.removed_files {
            vfs.remove_file(base_path.join(path))?;
        }

        if self.added_dirs.len() + self.added_files.len() > 0 {
            log::info!(
                "Wrote {} directories and {} files to the file system!",
                self.added_dirs.len(),
                self.added_files.len()
            );
        }
        if self.removed_dirs.len() + self.removed_files.len() > 0 {
            log::info!(
                "Removed {} directories and {} files from the file system. Yikes!",
                self.removed_dirs.len(),
                self.removed_files.len()
            );
        }
        Ok(())
    }
}

impl fmt::Debug for FsSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let files = self
            .added_files
            .iter()
            .map(|(k, v)| format!("{}: {} bytes", k.display(), v.len()));
        let dirs = self.added_dirs.iter().map(|v| format!("{}", v.display()));

        f.debug_list().entries(files).entries(dirs).finish()
    }
}
