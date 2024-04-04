use std::{
    collections::{HashMap, HashSet},
    io,
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
        let mut lock = vfs.lock();

        let base_path = base.as_ref();
        for dir_path in &self.added_dirs {
            match lock.create_dir_all(base_path.join(dir_path)) {
                Ok(_) => (),
                Err(err) if err.kind() == io::ErrorKind::AlreadyExists => (),
                Err(err) => return Err(err),
            };
        }
        for (path, contents) in &self.added_files {
            lock.write(base_path.join(path), contents)?;
        }
        for dir_path in &self.removed_dirs {
            lock.remove_dir_all(base_path.join(dir_path))?;
        }
        for path in &self.removed_files {
            lock.remove_file(base_path.join(path))?;
        }
        drop(lock);

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

    /// Returns whether this `FsSnapshot` is empty or not.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.added_files.is_empty()
            && self.added_dirs.is_empty()
            && self.removed_files.is_empty()
            && self.removed_dirs.is_empty()
    }

    /// Returns a list of paths that would be added by this `FsSnapshot`.
    #[inline]
    pub fn added_paths(&self) -> Vec<&Path> {
        let mut list = Vec::with_capacity(self.added_files.len() + self.added_dirs.len());
        list.extend(self.added_files.keys().map(PathBuf::as_path));
        list.extend(self.added_dirs.iter().map(PathBuf::as_path));

        list
    }

    /// Returns a list of paths that would be removed by this `FsSnapshot`.
    #[inline]
    pub fn removed_paths(&self) -> Vec<&Path> {
        let mut list = Vec::with_capacity(self.removed_files.len() + self.removed_dirs.len());
        list.extend(self.removed_files.iter().map(PathBuf::as_path));
        list.extend(self.removed_dirs.iter().map(PathBuf::as_path));

        list
    }

    /// Returns a list of file paths that would be added by this `FsSnapshot`
    #[inline]
    pub fn added_files(&self) -> Vec<&Path> {
        self.added_files.keys().map(PathBuf::as_path).collect()
    }

    /// Returns a list of directory paths that would be added by this `FsSnapshot`
    #[inline]
    pub fn added_dirs(&self) -> Vec<&Path> {
        self.added_dirs.iter().map(PathBuf::as_path).collect()
    }

    /// Returns a list of file paths that would be removed by this `FsSnapshot`
    #[inline]
    pub fn removed_files(&self) -> Vec<&Path> {
        self.removed_files.iter().map(PathBuf::as_path).collect()
    }

    /// Returns a list of directory paths that would be removed by this `FsSnapshot`
    #[inline]
    pub fn removed_dirs(&self) -> Vec<&Path> {
        self.removed_dirs.iter().map(PathBuf::as_path).collect()
    }
}
