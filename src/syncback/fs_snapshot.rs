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

    pub fn merge(&mut self, other: Self) {
        self.dir.extend(other.dir);
        self.files.extend(other.files);
    }

    pub fn push_file<P: AsRef<Path>>(&mut self, path: P, data: Vec<u8>) {
        self.files
            .insert(path.as_ref().to_path_buf(), Arc::new(data));
    }

    pub fn pop_dir<P: AsRef<Path>>(&mut self, path: P) -> bool {
        self.dir.remove(path.as_ref())
    }

    pub fn write_to_vfs<P: AsRef<Path>>(&self, base: P, vfs: &Vfs) -> io::Result<()> {
        let base_path = base.as_ref();
        let mut dirs = 0;
        let mut files = 0;
        for dir_path in &self.dir {
            match vfs.create_dir(base_path.join(dir_path)) {
                Ok(_) => (),
                Err(err) if err.kind() == io::ErrorKind::AlreadyExists => (),
                Err(err) => return Err(err),
            };
            dirs += 1;
        }
        for (path, contents) in &self.files {
            vfs.write(base_path.join(path), contents.as_slice())?;
            files += 1;
        }

        log::info!("Wrote {dirs} directories and {files} files to the file system!");
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
