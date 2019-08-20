use std::{
    io,
    path::{Path, PathBuf},
};

use crate::path_map::PathMap;

use super::{
    snapshot::ImfsSnapshot,
    error::{FsResult, FsError},
    fetcher::{ImfsFetcher, FileType},
};

/// An in-memory filesystem that can be incrementally populated and updated as
/// filesystem modification events occur.
///
/// All operations on the `Imfs` are lazy and do I/O as late as they can to
/// avoid reading extraneous files or directories from the disk. This means that
/// they all take `self` mutably, and means that it isn't possible to hold
/// references to the internal state of the Imfs while traversing it!
///
/// Most operations return `ImfsEntry` objects to work around this, which is
/// effectively a index into the `Imfs`.
pub struct Imfs<F> {
    inner: PathMap<ImfsItem>,
    fetcher: F,
}

impl<F: ImfsFetcher> Imfs<F> {
    pub fn new(fetcher: F) -> Imfs<F> {
        Imfs {
            inner: PathMap::new(),
            fetcher,
        }
    }

    pub fn load_from_snapshot(&mut self, path: impl AsRef<Path>, snapshot: ImfsSnapshot) {
        let path = path.as_ref();

        match snapshot {
            ImfsSnapshot::File(file) => {
                self.inner.insert(path.to_path_buf(), ImfsItem::File(ImfsFile {
                    path: path.to_path_buf(),
                    contents: Some(file.contents),
                }));
            }
            ImfsSnapshot::Directory(directory) => {
                self.inner.insert(path.to_path_buf(), ImfsItem::Directory(ImfsDirectory {
                    path: path.to_path_buf(),
                    children_enumerated: true,
                }));

                for (child_name, child) in directory.children.into_iter() {
                    self.load_from_snapshot(path.join(child_name), child);
                }
            }
        }
    }

    pub fn raise_file_change(&mut self, path: impl AsRef<Path>) -> FsResult<()> {
        let path = path.as_ref();

        if !self.would_be_resident(path) {
            return Ok(());
        }

        let new_type = self.fetcher.file_type(path)
            .map_err(|err| FsError::new(err, path.to_path_buf()))?;

        match self.inner.get_mut(path) {
            Some(existing_item) => {
                match (existing_item, &new_type) {
                    (ImfsItem::File(existing_file), FileType::File) => {
                        // Invalidate the existing file contents.
                        // We can probably be smarter about this by reading the changed file.
                        existing_file.contents = None;
                    }
                    (ImfsItem::Directory(_), FileType::Directory) => {
                        // No changes required, a directory updating doesn't mean anything to us.
                    }
                    (ImfsItem::File(_), FileType::Directory) => {
                        self.inner.remove(path);
                        self.inner.insert(path.to_path_buf(), ImfsItem::new_from_type(FileType::Directory, path));
                    }
                    (ImfsItem::Directory(_), FileType::File) => {
                        self.inner.remove(path);
                        self.inner.insert(path.to_path_buf(), ImfsItem::new_from_type(FileType::File, path));
                    }
                }
            }
            None => {
                self.inner.insert(path.to_path_buf(), ImfsItem::new_from_type(new_type, path));
            }
        }

        Ok(())
    }

    pub fn raise_file_removed(&mut self, path: impl AsRef<Path>) -> FsResult<()> {
        let path = path.as_ref();

        if !self.would_be_resident(path) {
            return Ok(());
        }

        self.inner.remove(path);
        Ok(())
    }

    pub fn get(&mut self, path: impl AsRef<Path>) -> FsResult<ImfsEntry> {
        self.read_if_not_exists(path.as_ref())?;

        let item = self.inner.get(path.as_ref()).unwrap();

        let is_file = match item {
            ImfsItem::File(_) => true,
            ImfsItem::Directory(_) => false,
        };

        Ok(ImfsEntry {
            path: item.path().to_path_buf(),
            is_file,
        })
    }

    pub fn get_contents(&mut self, path: impl AsRef<Path>) -> FsResult<&[u8]> {
        let path = path.as_ref();

        self.read_if_not_exists(path)?;

        match self.inner.get_mut(path).unwrap() {
            ImfsItem::File(file) => {
                if file.contents.is_none() {
                    file.contents = Some(self.fetcher.read_contents(path)
                        .map_err(|err| FsError::new(err, path.to_path_buf()))?);
                }

                Ok(file.contents.as_ref().unwrap())
            }
            ImfsItem::Directory(_) => Err(FsError::new(io::Error::new(io::ErrorKind::Other, "Can't read a directory"), path.to_path_buf()))
        }
    }

    pub fn get_children(&mut self, path: impl AsRef<Path>) -> FsResult<Vec<ImfsEntry>> {
        let path = path.as_ref();

        self.read_if_not_exists(path)?;

        match self.inner.get(path).unwrap() {
            ImfsItem::Directory(dir) => {
                if dir.children_enumerated {
                    return self.inner.children(path)
                        .unwrap() // TODO: Handle None here, which means the PathMap entry did not exist.
                        .into_iter()
                        .map(PathBuf::from) // Convert paths from &Path to PathBuf
                        .collect::<Vec<PathBuf>>() // Collect all PathBufs, since self.get needs to borrow self mutably.
                        .into_iter()
                        .map(|path| self.get(path))
                        .collect::<FsResult<Vec<ImfsEntry>>>();
                }

                self.fetcher.read_children(path)
                    .map_err(|err| FsError::new(err, path.to_path_buf()))?
                    .into_iter()
                    .map(|path| self.get(path))
                    .collect::<FsResult<Vec<ImfsEntry>>>()
            }
            ImfsItem::File(_) => Err(FsError::new(io::Error::new(io::ErrorKind::Other, "Can't read a directory"), path.to_path_buf()))
        }
    }

    /// Tells whether the given path, if it were loaded, would be loaded if it
    /// existed.
    ///
    /// Returns true if the path is loaded or if its parent is loaded, is a
    /// directory, and is marked as having been enumerated before.
    ///
    /// This idea corresponds to whether a file change event should result in
    /// tangible changes to the in-memory filesystem. If a path would be
    /// resident, we need to read it, and if its contents were known before, we
    /// need to update them.
    fn would_be_resident(&self, path: &Path) -> bool {
        if self.inner.contains_key(path) {
            return true;
        }

        if let Some(parent) = path.parent() {
            if let Some(ImfsItem::Directory(dir)) = self.inner.get(parent) {
                return !dir.children_enumerated;
            }
        }

        false
    }

    /// Attempts to read the path into the `Imfs` if it doesn't exist.
    ///
    /// This does not necessitate that file contents or directory children will
    /// be read. Depending on the `ImfsFetcher` implementation that the `Imfs`
    /// is using, this call may read exactly only the given path and no more.
    fn read_if_not_exists(&mut self, path: &Path) -> FsResult<()> {
        if !self.inner.contains_key(path) {
            let kind = self.fetcher.file_type(path)
                .map_err(|err| FsError::new(err, path.to_path_buf()))?;

            self.inner.insert(path.to_path_buf(), ImfsItem::new_from_type(kind, path));
        }

        Ok(())
    }
}

/// A reference to file or folder in an `Imfs`. Can only be produced by the
/// entry existing in the Imfs, but can later point to nothing if something
/// would invalidate that path.
///
/// This struct does not borrow from the Imfs since every operation has the
/// possibility to mutate the underlying data structure and move memory around.
pub struct ImfsEntry {
    path: PathBuf,
    is_file: bool,
}

impl ImfsEntry {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn contents<'imfs>(
        &self,
        imfs: &'imfs mut Imfs<impl ImfsFetcher>,
    ) -> FsResult<&'imfs [u8]> {
        imfs.get_contents(&self.path)
    }

    pub fn children(
        &self,
        imfs: &mut Imfs<impl ImfsFetcher>,
    ) -> FsResult<Vec<ImfsEntry>> {
        imfs.get_children(&self.path)
    }

    pub fn is_file(&self) -> bool {
        self.is_file
    }

    pub fn is_directory(&self) -> bool {
        !self.is_file
    }
}

/// Internal structure describing potentially partially-resident files and
/// folders in the `Imfs`.
pub enum ImfsItem {
    File(ImfsFile),
    Directory(ImfsDirectory),
}

impl ImfsItem {
    fn path(&self) -> &Path {
        match self {
            ImfsItem::File(file) => &file.path,
            ImfsItem::Directory(dir) => &dir.path,
        }
    }

    fn new_from_type(kind: FileType, path: impl Into<PathBuf>) -> ImfsItem {
        match kind {
            FileType::Directory => ImfsItem::Directory(ImfsDirectory {
                path: path.into(),
                children_enumerated: false,
            }),
            FileType::File => ImfsItem::File(ImfsFile {
                path: path.into(),
                contents: None,
            }),
        }
    }
}

pub struct ImfsFile {
    pub(super) path: PathBuf,
    pub(super) contents: Option<Vec<u8>>,
}

pub struct ImfsDirectory {
    pub(super) path: PathBuf,
    pub(super) children_enumerated: bool,
}

#[cfg(test)]
mod test {
    use super::*;

    use std::{
        rc::Rc,
        cell::RefCell,
    };

    use maplit::hashmap;

    use super::super::{
        noop_fetcher::NoopFetcher,
        error::FsErrorKind,
    };

    #[test]
    fn from_snapshot_file() {
        let mut imfs = Imfs::new(NoopFetcher);
        let file = ImfsSnapshot::file("hello, world!");

        imfs.load_from_snapshot("/hello.txt", file);

        let entry = imfs.get_contents("/hello.txt").unwrap();
        assert_eq!(entry, b"hello, world!");
    }

    #[test]
    fn from_snapshot_dir() {
        let mut imfs = Imfs::new(NoopFetcher);
        let dir = ImfsSnapshot::dir(hashmap! {
            "a.txt" => ImfsSnapshot::file("contents of a.txt"),
            "b.lua" => ImfsSnapshot::file("contents of b.lua"),
        });

        imfs.load_from_snapshot("/dir", dir);

        let children = imfs.get_children("/dir").unwrap();

        let mut has_a = false;
        let mut has_b = false;

        for child in children.into_iter() {
            if child.path() == Path::new("/dir/a.txt") {
                has_a = true;
            } else if child.path() == Path::new("/dir/b.lua") {
                has_b = true;
            } else {
                panic!("Unexpected child in /dir");
            }
        }

        assert!(has_a, "/dir/a.txt was missing");
        assert!(has_b, "/dir/b.lua was missing");

        let a = imfs.get_contents("/dir/a.txt").unwrap();
        assert_eq!(a, b"contents of a.txt");

        let b = imfs.get_contents("/dir/b.lua").unwrap();
        assert_eq!(b, b"contents of b.lua");
    }

    #[test]
    fn changed_event() {
        #[derive(Default)]
        struct MockState {
            a_contents: &'static str,
        }

        struct MockFetcher {
            inner: Rc<RefCell<MockState>>,
        }

        impl ImfsFetcher for MockFetcher {
            fn file_type(&mut self, path: &Path) -> io::Result<FileType> {
                if path == Path::new("/dir/a.txt") {
                    return Ok(FileType::File);
                }

                unimplemented!();
            }

            fn read_contents(&mut self, path: &Path) -> io::Result<Vec<u8>> {
                if path == Path::new("/dir/a.txt") {
                    let inner = self.inner.borrow();

                    return Ok(Vec::from(inner.a_contents));
                }

                unimplemented!();
            }

            fn read_children(&mut self, _path: &Path) -> io::Result<Vec<PathBuf>> {
                unimplemented!();
            }

            fn create_directory(&mut self, _path: &Path) -> io::Result<()> {
                unimplemented!();
            }

            fn write_file(&mut self, _path: &Path, _contents: &[u8]) -> io::Result<()> {
                unimplemented!();
            }

            fn remove(&mut self, _path: &Path) -> io::Result<()> {
                unimplemented!();
            }
        }

        let mock_state = Rc::new(RefCell::new(MockState {
            a_contents: "Initial contents",
        }));

        let mut imfs = Imfs::new(MockFetcher {
            inner: mock_state.clone(),
        });

        let a = imfs.get("/dir/a.txt")
            .expect("mock file did not exist");

        let contents = a.contents(&mut imfs)
            .expect("mock file contents error");

        assert_eq!(contents, b"Initial contents");

        {
            let mut mock_state = mock_state.borrow_mut();
            mock_state.a_contents = "Changed contents";
        }

        imfs.raise_file_change("/dir/a.txt")
            .expect("error processing file change");

        let contents = a.contents(&mut imfs)
            .expect("mock file contents error");

        assert_eq!(contents, b"Changed contents");
    }

    #[test]
    fn removed_event_existing() {
        let mut imfs = Imfs::new(NoopFetcher);

        let file = ImfsSnapshot::file("hello, world!");
        imfs.load_from_snapshot("/hello.txt", file);

        let hello = imfs.get("/hello.txt")
            .expect("couldn't get hello.txt");

        let contents = hello.contents(&mut imfs)
            .expect("couldn't get hello.txt contents");

        assert_eq!(contents, b"hello, world!");

        imfs.raise_file_removed("/hello.txt")
            .expect("error processing file removal");

        match imfs.get("hello.txt") {
            Err(ref err) if err.kind() == FsErrorKind::NotFound => {}
            Ok(_) => {
                panic!("hello.txt was not removed from Imfs");
            }
            Err(err) => {
                panic!("Unexpected error: {:?}", err);
            }
        }
    }
}