use std::{
    io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use crossbeam_channel::Receiver;

use crate::path_map::PathMap;

use super::{
    error::{FsError, FsResult},
    event::VfsEvent,
    fetcher::{FileType, VfsFetcher},
    snapshot::VfsSnapshot,
};

/// An in-memory filesystem that can be incrementally populated and updated as
/// filesystem modification events occur.
///
/// All operations on the `Vfs` are lazy and do I/O as late as they can to
/// avoid reading extraneous files or directories from the disk. This means that
/// they all take `self` mutably, and means that it isn't possible to hold
/// references to the internal state of the Vfs while traversing it!
///
/// Most operations return `VfsEntry` objects to work around this, which is
/// effectively a index into the `Vfs`.
pub struct Vfs<F> {
    /// A hierarchical map from paths to items that have been read or partially
    /// read into memory by the Vfs.
    data: Mutex<PathMap<VfsItem>>,

    /// This Vfs's fetcher, which is used for all actual interactions with the
    /// filesystem. It's referred to by the type parameter `F` all over, and is
    /// generic in order to make it feasible to mock.
    fetcher: F,
}

impl<F: VfsFetcher> Vfs<F> {
    pub fn new(fetcher: F) -> Self {
        Self {
            data: Mutex::new(PathMap::new()),
            fetcher,
        }
    }

    pub fn change_receiver(&self) -> Receiver<VfsEvent> {
        self.fetcher.receiver()
    }

    pub fn commit_change(&self, event: &VfsEvent) -> FsResult<()> {
        use VfsEvent::*;

        log::trace!("Committing Vfs change {:?}", event);

        let mut data = self.data.lock().unwrap();

        match event {
            Created(path) | Modified(path) => {
                Self::raise_file_changed(&mut data, &self.fetcher, path)?;
            }
            Removed(path) => {
                Self::raise_file_removed(&mut data, &self.fetcher, path)?;
            }
        }

        Ok(())
    }

    pub fn get(&self, path: impl AsRef<Path>) -> FsResult<VfsEntry> {
        let mut data = self.data.lock().unwrap();
        Self::get_internal(&mut data, &self.fetcher, path)
    }

    pub fn get_contents(&self, path: impl AsRef<Path>) -> FsResult<Arc<Vec<u8>>> {
        let path = path.as_ref();

        let mut data = self.data.lock().unwrap();
        Self::read_if_not_exists(&mut data, &self.fetcher, path)?;

        match data.get_mut(path).unwrap() {
            VfsItem::File(file) => {
                if file.contents.is_none() {
                    file.contents = Some(
                        self.fetcher
                            .read_contents(path)
                            .map(Arc::new)
                            .map_err(|err| FsError::new(err, path.to_path_buf()))?,
                    );
                }

                Ok(file.contents.clone().unwrap())
            }
            VfsItem::Directory(_) => Err(FsError::new(
                io::Error::new(io::ErrorKind::Other, "Can't read a directory"),
                path.to_path_buf(),
            )),
        }
    }

    pub fn get_children(&self, path: impl AsRef<Path>) -> FsResult<Vec<VfsEntry>> {
        let path = path.as_ref();

        let mut data = self.data.lock().unwrap();
        Self::read_if_not_exists(&mut data, &self.fetcher, path)?;

        match data.get_mut(path).unwrap() {
            VfsItem::Directory(dir) => {
                self.fetcher.watch(path);

                // If the directory hasn't been marked as enumerated yet, find
                // all of its children and insert them into the VFS.
                if !dir.children_enumerated {
                    dir.children_enumerated = true;

                    let children = self
                        .fetcher
                        .read_children(path)
                        .map_err(|err| FsError::new(err, path.to_path_buf()))?;

                    for path in children {
                        Self::get_internal(&mut data, &self.fetcher, path)?;
                    }
                }

                data.children(path)
                    .unwrap() // TODO: Handle None here, which means the PathMap entry did not exist.
                    .into_iter()
                    .map(PathBuf::from) // Convert paths from &Path to PathBuf
                    .collect::<Vec<PathBuf>>() // Collect all PathBufs, since self.get needs to borrow self mutably.
                    .into_iter()
                    .map(|path| Self::get_internal(&mut data, &self.fetcher, path))
                    .collect::<FsResult<Vec<VfsEntry>>>()
            }
            VfsItem::File(_) => Err(FsError::new(
                io::Error::new(io::ErrorKind::Other, "Can't read a directory"),
                path.to_path_buf(),
            )),
        }
    }

    fn get_internal(
        data: &mut PathMap<VfsItem>,
        fetcher: &F,
        path: impl AsRef<Path>,
    ) -> FsResult<VfsEntry> {
        let path = path.as_ref();

        Self::read_if_not_exists(data, fetcher, path)?;

        let item = data.get(path).unwrap();

        let is_file = match item {
            VfsItem::File(_) => true,
            VfsItem::Directory(_) => false,
        };

        Ok(VfsEntry {
            path: item.path().to_path_buf(),
            is_file,
        })
    }

    fn raise_file_changed(
        data: &mut PathMap<VfsItem>,
        fetcher: &F,
        path: impl AsRef<Path>,
    ) -> FsResult<()> {
        let path = path.as_ref();

        if !Self::would_be_resident(&data, path) {
            log::trace!(
                "Path would not be resident, skipping change: {}",
                path.display()
            );

            return Ok(());
        }

        let new_type = fetcher
            .file_type(path)
            .map_err(|err| FsError::new(err, path.to_path_buf()))?;

        match data.get_mut(path) {
            Some(existing_item) => {
                match (existing_item, &new_type) {
                    (VfsItem::File(existing_file), FileType::File) => {
                        // Invalidate the existing file contents.
                        // We can probably be smarter about this by reading the changed file.
                        existing_file.contents = None;
                    }
                    (VfsItem::Directory(_), FileType::Directory) => {
                        // No changes required, a directory updating doesn't mean anything to us.
                        fetcher.watch(path);
                    }
                    (VfsItem::File(_), FileType::Directory) => {
                        data.remove(path);
                        data.insert(
                            path.to_path_buf(),
                            VfsItem::new_from_type(FileType::Directory, path),
                        );
                        fetcher.watch(path);
                    }
                    (VfsItem::Directory(_), FileType::File) => {
                        data.remove(path);
                        data.insert(
                            path.to_path_buf(),
                            VfsItem::new_from_type(FileType::File, path),
                        );
                        fetcher.unwatch(path);
                    }
                }
            }
            None => {
                log::trace!("Inserting new path {}", path.display());
                data.insert(path.to_path_buf(), VfsItem::new_from_type(new_type, path));
            }
        }

        Ok(())
    }

    fn raise_file_removed(
        data: &mut PathMap<VfsItem>,
        fetcher: &F,
        path: impl AsRef<Path>,
    ) -> FsResult<()> {
        let path = path.as_ref();

        if !Self::would_be_resident(data, path) {
            return Ok(());
        }

        data.remove(path);
        fetcher.unwatch(path);
        Ok(())
    }

    /// Attempts to read the path into the `Vfs` if it doesn't exist.
    ///
    /// This does not necessitate that file contents or directory children will
    /// be read. Depending on the `VfsFetcher` implementation that the `Vfs`
    /// is using, this call may read exactly only the given path and no more.
    fn read_if_not_exists(data: &mut PathMap<VfsItem>, fetcher: &F, path: &Path) -> FsResult<()> {
        if !data.contains_key(path) {
            let kind = fetcher
                .file_type(path)
                .map_err(|err| FsError::new(err, path.to_path_buf()))?;

            if kind == FileType::Directory {
                fetcher.watch(path);
            }

            data.insert(path.to_path_buf(), VfsItem::new_from_type(kind, path));
        }

        Ok(())
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
    fn would_be_resident(data: &PathMap<VfsItem>, path: &Path) -> bool {
        if data.contains_key(path) {
            return true;
        }

        if let Some(parent) = path.parent() {
            if let Some(VfsItem::Directory(dir)) = data.get(parent) {
                return dir.children_enumerated;
            }
        }

        false
    }
}

/// Contains extra methods that should only be used for debugging. They're
/// broken out into a separate trait to make it more explicit to depend on them.
pub trait VfsDebug {
    fn debug_load_snapshot<P: AsRef<Path>>(&self, path: P, snapshot: VfsSnapshot);
    fn debug_is_file(&self, path: &Path) -> bool;
    fn debug_contents(&self, path: &Path) -> Option<Arc<Vec<u8>>>;
    fn debug_children(&self, path: &Path) -> Option<(bool, Vec<PathBuf>)>;
    fn debug_orphans(&self) -> Vec<PathBuf>;
    fn debug_watched_paths(&self) -> Vec<PathBuf>;
}

impl<F: VfsFetcher> VfsDebug for Vfs<F> {
    fn debug_load_snapshot<P: AsRef<Path>>(&self, path: P, snapshot: VfsSnapshot) {
        fn load_snapshot<P: AsRef<Path>>(
            data: &mut PathMap<VfsItem>,
            path: P,
            snapshot: VfsSnapshot,
        ) {
            let path = path.as_ref();

            match snapshot {
                VfsSnapshot::File(file) => {
                    data.insert(
                        path.to_path_buf(),
                        VfsItem::File(VfsFile {
                            path: path.to_path_buf(),
                            contents: Some(Arc::new(file.contents)),
                        }),
                    );
                }
                VfsSnapshot::Directory(directory) => {
                    data.insert(
                        path.to_path_buf(),
                        VfsItem::Directory(VfsDirectory {
                            path: path.to_path_buf(),
                            children_enumerated: true,
                        }),
                    );

                    for (child_name, child) in directory.children.into_iter() {
                        load_snapshot(data, path.join(child_name), child);
                    }
                }
            }
        }

        let mut data = self.data.lock().unwrap();
        load_snapshot(&mut data, path, snapshot)
    }

    fn debug_is_file(&self, path: &Path) -> bool {
        let data = self.data.lock().unwrap();
        match data.get(path) {
            Some(VfsItem::File(_)) => true,
            _ => false,
        }
    }

    fn debug_contents(&self, path: &Path) -> Option<Arc<Vec<u8>>> {
        let data = self.data.lock().unwrap();
        match data.get(path) {
            Some(VfsItem::File(file)) => file.contents.clone(),
            _ => None,
        }
    }

    fn debug_children(&self, path: &Path) -> Option<(bool, Vec<PathBuf>)> {
        let data = self.data.lock().unwrap();
        match data.get(path) {
            Some(VfsItem::Directory(dir)) => Some((
                dir.children_enumerated,
                data.children(path)
                    .unwrap()
                    .iter()
                    .map(|path| path.to_path_buf())
                    .collect(),
            )),
            _ => None,
        }
    }

    fn debug_orphans(&self) -> Vec<PathBuf> {
        let data = self.data.lock().unwrap();
        data.orphans().map(|path| path.to_path_buf()).collect()
    }

    fn debug_watched_paths(&self) -> Vec<PathBuf> {
        self.fetcher.watched_paths()
    }
}

/// A reference to file or folder in an `Vfs`. Can only be produced by the
/// entry existing in the Vfs, but can later point to nothing if something
/// would invalidate that path.
///
/// This struct does not borrow from the Vfs since every operation has the
/// possibility to mutate the underlying data structure and move memory around.
pub struct VfsEntry {
    path: PathBuf,
    is_file: bool,
}

impl VfsEntry {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn contents(&self, vfs: &Vfs<impl VfsFetcher>) -> FsResult<Arc<Vec<u8>>> {
        vfs.get_contents(&self.path)
    }

    pub fn children(&self, vfs: &Vfs<impl VfsFetcher>) -> FsResult<Vec<VfsEntry>> {
        vfs.get_children(&self.path)
    }

    pub fn is_file(&self) -> bool {
        self.is_file
    }

    pub fn is_directory(&self) -> bool {
        !self.is_file
    }
}

/// Internal structure describing potentially partially-resident files and
/// folders in the `Vfs`.
#[derive(Debug)]
pub enum VfsItem {
    File(VfsFile),
    Directory(VfsDirectory),
}

impl VfsItem {
    fn path(&self) -> &Path {
        match self {
            VfsItem::File(file) => &file.path,
            VfsItem::Directory(dir) => &dir.path,
        }
    }

    fn new_from_type(kind: FileType, path: impl Into<PathBuf>) -> VfsItem {
        match kind {
            FileType::Directory => VfsItem::Directory(VfsDirectory {
                path: path.into(),
                children_enumerated: false,
            }),
            FileType::File => VfsItem::File(VfsFile {
                path: path.into(),
                contents: None,
            }),
        }
    }
}

#[derive(Debug)]
pub struct VfsFile {
    pub(super) path: PathBuf,
    pub(super) contents: Option<Arc<Vec<u8>>>,
}

#[derive(Debug)]
pub struct VfsDirectory {
    pub(super) path: PathBuf,
    pub(super) children_enumerated: bool,
}

#[cfg(test)]
mod test {
    use super::*;

    use std::{cell::RefCell, rc::Rc};

    use crossbeam_channel::Receiver;
    use maplit::hashmap;

    use super::super::{error::FsErrorKind, event::VfsEvent, noop_fetcher::NoopFetcher};

    #[test]
    fn from_snapshot_file() {
        let vfs = Vfs::new(NoopFetcher);
        let file = VfsSnapshot::file("hello, world!");

        vfs.debug_load_snapshot("/hello.txt", file);

        let contents = vfs.get_contents("/hello.txt").unwrap();
        assert_eq!(contents.as_slice(), b"hello, world!");
    }

    #[test]
    fn from_snapshot_dir() {
        let vfs = Vfs::new(NoopFetcher);
        let dir = VfsSnapshot::dir(hashmap! {
            "a.txt" => VfsSnapshot::file("contents of a.txt"),
            "b.lua" => VfsSnapshot::file("contents of b.lua"),
        });

        vfs.debug_load_snapshot("/dir", dir);

        let children = vfs.get_children("/dir").unwrap();

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

        let a_contents = vfs.get_contents("/dir/a.txt").unwrap();
        assert_eq!(a_contents.as_slice(), b"contents of a.txt");

        let b_contents = vfs.get_contents("/dir/b.lua").unwrap();
        assert_eq!(b_contents.as_slice(), b"contents of b.lua");
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

        impl VfsFetcher for MockFetcher {
            fn file_type(&self, path: &Path) -> io::Result<FileType> {
                if path == Path::new("/dir/a.txt") {
                    return Ok(FileType::File);
                }

                unimplemented!();
            }

            fn read_contents(&self, path: &Path) -> io::Result<Vec<u8>> {
                if path == Path::new("/dir/a.txt") {
                    let inner = self.inner.borrow();

                    return Ok(Vec::from(inner.a_contents));
                }

                unimplemented!();
            }

            fn read_children(&self, _path: &Path) -> io::Result<Vec<PathBuf>> {
                unimplemented!();
            }

            fn create_directory(&self, _path: &Path) -> io::Result<()> {
                unimplemented!();
            }

            fn write_file(&self, _path: &Path, _contents: &[u8]) -> io::Result<()> {
                unimplemented!();
            }

            fn remove(&self, _path: &Path) -> io::Result<()> {
                unimplemented!();
            }

            fn receiver(&self) -> Receiver<VfsEvent> {
                crossbeam_channel::never()
            }
        }

        let mock_state = Rc::new(RefCell::new(MockState {
            a_contents: "Initial contents",
        }));

        let mut vfs = Vfs::new(MockFetcher {
            inner: mock_state.clone(),
        });

        let a = vfs.get("/dir/a.txt").expect("mock file did not exist");

        let contents = a.contents(&mut vfs).expect("mock file contents error");

        assert_eq!(contents.as_slice(), b"Initial contents");

        {
            let mut mock_state = mock_state.borrow_mut();
            mock_state.a_contents = "Changed contents";
        }

        vfs.commit_change(&VfsEvent::Modified(PathBuf::from("/dir/a.txt")))
            .expect("error processing file change");

        let contents = a.contents(&mut vfs).expect("mock file contents error");

        assert_eq!(contents.as_slice(), b"Changed contents");
    }

    #[test]
    fn removed_event_existing() {
        let mut vfs = Vfs::new(NoopFetcher);

        let file = VfsSnapshot::file("hello, world!");
        vfs.debug_load_snapshot("/hello.txt", file);

        let hello = vfs.get("/hello.txt").expect("couldn't get hello.txt");

        let contents = hello
            .contents(&mut vfs)
            .expect("couldn't get hello.txt contents");

        assert_eq!(contents.as_slice(), b"hello, world!");

        vfs.commit_change(&VfsEvent::Removed(PathBuf::from("/hello.txt")))
            .expect("error processing file removal");

        match vfs.get("hello.txt") {
            Err(ref err) if err.kind() == FsErrorKind::NotFound => {}
            Ok(_) => {
                panic!("hello.txt was not removed from Vfs");
            }
            Err(err) => {
                panic!("Unexpected error: {:?}", err);
            }
        }
    }
}
