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
    add_files: HashMap<PathBuf, Vec<u8>>,
    /// Paths representing new directories.
    add_dirs: HashSet<PathBuf>,
    /// Paths representing removed files.
    removed_files: HashSet<PathBuf>,
    /// Paths representing removed directories.
    removed_dirs: HashSet<PathBuf>,
}

impl FsSnapshot {
    /// Creates a new `FsSnapshot`.
    pub fn new() -> Self {
        Self {
            add_files: HashMap::new(),
            add_dirs: HashSet::new(),
            removed_files: HashSet::new(),
            removed_dirs: HashSet::new(),
        }
    }

    /// Adds the given path to the `FsSnapshot` as a file with the given
    /// contents, then returns it.
    pub fn with_added_file<P: AsRef<Path>>(mut self, path: P, data: Vec<u8>) -> Self {
        self.add_files.insert(path.as_ref().to_path_buf(), data);
        self
    }

    /// Adds the given path to the `FsSnapshot` as a file with the given
    /// then returns it.
    pub fn with_added_dir<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.add_dirs.insert(path.as_ref().to_path_buf());
        self
    }

    /// Merges two `FsSnapshot`s together.
    #[inline]
    pub fn merge(&mut self, other: Self) {
        self.add_files.extend(other.add_files);
        self.add_dirs.extend(other.add_dirs);
        self.removed_files.extend(other.removed_files);
        self.removed_dirs.extend(other.removed_dirs);
    }

    /// Adds the provided path as a file with the given contents.
    pub fn add_file<P: AsRef<Path>>(&mut self, path: P, data: Vec<u8>) {
        self.add_files.insert(path.as_ref().to_path_buf(), data);
    }

    /// Adds the provided path as a directory.
    pub fn add_dir<P: AsRef<Path>>(&mut self, path: P) {
        self.add_dirs.insert(path.as_ref().to_path_buf());
    }

    /// Writes the `FsSnapshot` to the provided VFS, using the provided `base`
    /// as a root for the other paths in the `FsSnapshot`.
    pub fn write_to_vfs<P: AsRef<Path>>(&self, base: P, vfs: &Vfs) -> io::Result<()> {
        let base_path = base.as_ref();
        let mut dirs = 0;
        let mut files = 0;
        for dir_path in &self.add_dirs {
            match vfs.create_dir_all(base_path.join(dir_path)) {
                Ok(_) => (),
                Err(err) if err.kind() == io::ErrorKind::AlreadyExists => (),
                Err(err) => return Err(err),
            };
            dirs += 1;
        }
        for (path, contents) in &self.add_files {
            vfs.write(base_path.join(path), contents)?;
            files += 1;
        }

        log::info!("Wrote {dirs} directories and {files} files to the file system!");
        Ok(())
    }
}

impl fmt::Debug for FsSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let files = self
            .add_files
            .iter()
            .map(|(k, v)| format!("{}: {} bytes", k.display(), v.len()));
        let dirs = self.add_dirs.iter().map(|v| format!("{}", v.display()));

        f.debug_list().entries(files).entries(dirs).finish()
    }
}
