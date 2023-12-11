use std::{
    collections::{HashMap, HashSet},
    fmt, io,
    path::{Path, PathBuf},
    sync::Arc,
};

use memofs::Vfs;

pub struct FsSnapshot {
    files: HashMap<PathBuf, Arc<Vec<u8>>>,
    dir: HashSet<PathBuf>,
}

impl FsSnapshot {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            dir: HashSet::new(),
        }
    }

    pub fn with_file<P: AsRef<Path>>(mut self, path: P, data: Vec<u8>) -> Self {
        self.files
            .insert(path.as_ref().to_path_buf(), Arc::new(data));
        self
    }

    pub fn with_dir<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.dir.insert(path.as_ref().to_path_buf());
        self
    }

    pub fn push_file<P: AsRef<Path>>(&mut self, path: P, data: Vec<u8>) {
        self.files
            .insert(path.as_ref().to_path_buf(), Arc::new(data));
    }

    pub fn write_to_vfs(&self, vfs: &Vfs) -> io::Result<()> {
        for dir_path in &self.dir {
            vfs.create_dir(dir_path)?;
        }
        for (path, contents) in &self.files {
            vfs.write(path, contents.as_slice())?;
        }

        Ok(())
    }
}

impl fmt::Debug for FsSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let files = self
            .files
            .iter()
            .map(|(k, v)| format!("{}: {} bytes", k.display(), v.len()));
        let dirs = self.dir.iter().map(|v| format!("{}", v.display()));

        f.debug_list().entries(files).entries(dirs).finish()
    }
}
