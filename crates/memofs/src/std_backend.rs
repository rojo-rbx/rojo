use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::{collections::HashSet, io};

use crossbeam_channel::Receiver;
use notify::{watcher, DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};

use crate::{DirEntry, Metadata, ReadDir, VfsBackend, VfsEvent};

/// `VfsBackend` that uses `std::fs` and the `notify` crate.
pub struct StdBackend {
    watcher: Mutex<RecommendedWatcher>,
    watcher_receiver: Receiver<VfsEvent>,
    watches: Mutex<HashSet<PathBuf>>,
    cache: Arc<Mutex<Cache>>,
}

/// Snapshotting asks about many paths that usually don't exist, like `init.*`
/// and `*.meta.json` files. Answering those from one directory listing instead
/// of a syscall each is much faster, especially on Windows. The cache is
/// cleared whenever anything on disk changes.
#[derive(Default)]
struct Cache {
    listings: HashMap<PathBuf, Arc<Listing>>,
    canonical: HashMap<PathBuf, PathBuf>,
}

struct Listing {
    paths: Vec<PathBuf>,
    kinds: HashMap<OsString, Kind>,
    lowercase_names: HashSet<String>,
}

#[derive(Clone, Copy)]
enum Kind {
    File,
    Dir,
    /// Symlinks (including Windows junctions) and anything else that needs a
    /// real syscall to resolve.
    Other,
}

impl StdBackend {
    pub fn new() -> io::Result<StdBackend> {
        let (notify_tx, notify_rx) = mpsc::channel();
        let watcher = watcher(notify_tx, Duration::from_millis(50)).map_err(io::Error::other)?;

        let (tx, rx) = crossbeam_channel::unbounded();
        let cache = Arc::new(Mutex::new(Cache::default()));
        let thread_cache = Arc::clone(&cache);

        thread::spawn(move || {
            for event in notify_rx {
                *thread_cache.lock().unwrap() = Cache::default();

                match event {
                    DebouncedEvent::Create(path) => {
                        tx.send(VfsEvent::Create(path))?;
                    }
                    DebouncedEvent::Write(path) => {
                        tx.send(VfsEvent::Write(path))?;
                    }
                    DebouncedEvent::Remove(path) => {
                        tx.send(VfsEvent::Remove(path))?;
                    }
                    DebouncedEvent::Rename(from, to) => {
                        tx.send(VfsEvent::Remove(from))?;
                        tx.send(VfsEvent::Create(to))?;
                    }
                    _ => {}
                }
            }

            Result::<(), crossbeam_channel::SendError<VfsEvent>>::Ok(())
        });

        Ok(Self {
            watcher: Mutex::new(watcher),
            watcher_receiver: rx,
            watches: Mutex::new(HashSet::new()),
            cache,
        })
    }

    fn clear_cache(&self) {
        *self.cache.lock().unwrap() = Cache::default();
    }

    fn listing(&self, dir: &Path) -> io::Result<Arc<Listing>> {
        if let Some(listing) = self.cache.lock().unwrap().listings.get(dir) {
            return Ok(Arc::clone(listing));
        }

        let mut entries = fs_err::read_dir(dir)?.collect::<io::Result<Vec<_>>>()?;
        entries.sort_by_cached_key(|entry| entry.file_name());

        let listing = Arc::new(Listing {
            paths: entries.iter().map(|entry| entry.path()).collect(),
            kinds: entries
                .iter()
                .map(|entry| {
                    let kind = match entry.file_type() {
                        Ok(file_type) if file_type.is_dir() => Kind::Dir,
                        Ok(file_type) if file_type.is_file() => Kind::File,
                        _ => Kind::Other,
                    };
                    (entry.file_name(), kind)
                })
                .collect(),
            lowercase_names: entries
                .iter()
                .filter_map(|entry| entry.file_name().to_str().map(str::to_lowercase))
                .collect(),
        });

        self.cache
            .lock()
            .unwrap()
            .listings
            .insert(dir.to_path_buf(), Arc::clone(&listing));

        Ok(listing)
    }

    /// Looks `path` up in its parent's listing, listing the parent first if
    /// `list_parent` is set. Returns `None` if only a syscall can tell, and
    /// `Some(None)` if `path` doesn't exist.
    fn lookup(&self, path: &Path, list_parent: bool) -> Option<Option<Kind>> {
        let (parent, name) = (path.parent()?, path.file_name()?);
        let listing = if list_parent {
            self.listing(parent).ok()?
        } else {
            Arc::clone(self.cache.lock().unwrap().listings.get(parent)?)
        };

        if let Some(kind) = listing.kinds.get(name) {
            return Some(Some(*kind));
        }

        // Case-insensitive filesystems, Unicode normalization, and Windows'
        // short names can all resolve a name that isn't listed verbatim.
        let name = name.to_str()?;
        if !name.is_ascii()
            || name.contains('~')
            || listing.lowercase_names.contains(&name.to_lowercase())
        {
            return None;
        }

        Some(None)
    }
}

fn not_found(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::NotFound,
        format!("{} does not exist", path.display()),
    )
}

/// Returns what `dunce::canonicalize` would for `name` inside the already
/// canonical `parent`, which keeps the `\\?\` prefix on Windows when a path is
/// too long or unusual to go without it.
fn join_canonical(parent: &Path, name: &OsStr) -> PathBuf {
    let path = parent.join(name);

    if cfg!(windows) && !path.as_os_str().as_encoded_bytes().starts_with(br"\\?\") {
        let mut verbatim = OsString::from(r"\\?\");
        verbatim.push(path.as_os_str());
        return dunce::simplified(Path::new(&verbatim)).to_path_buf();
    }

    path
}

impl VfsBackend for StdBackend {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        if let Some(None) = self.lookup(path, true) {
            return Err(not_found(path));
        }

        fs_err::read(path)
    }

    fn write(&mut self, path: &Path, data: &[u8]) -> io::Result<()> {
        self.clear_cache();
        fs_err::write(path, data)
    }

    fn exists(&self, path: &Path) -> io::Result<bool> {
        match self.lookup(path, true) {
            Some(Some(Kind::File | Kind::Dir)) => Ok(true),
            Some(None) => Ok(false),
            _ => std::fs::exists(path),
        }
    }

    fn read_dir(&self, path: &Path) -> io::Result<ReadDir> {
        let inner = self
            .listing(path)?
            .paths
            .clone()
            .into_iter()
            .map(|path| Ok(DirEntry { path }));

        Ok(ReadDir {
            inner: Box::new(inner),
        })
    }

    fn create_dir(&mut self, path: &Path) -> io::Result<()> {
        self.clear_cache();
        fs_err::create_dir(path)
    }

    fn create_dir_all(&mut self, path: &Path) -> io::Result<()> {
        self.clear_cache();
        fs_err::create_dir_all(path)
    }

    fn remove_file(&mut self, path: &Path) -> io::Result<()> {
        self.clear_cache();
        fs_err::remove_file(path)
    }

    fn remove_dir_all(&mut self, path: &Path) -> io::Result<()> {
        self.clear_cache();
        fs_err::remove_dir_all(path)
    }

    fn metadata(&self, path: &Path) -> io::Result<Metadata> {
        match self.lookup(path, true) {
            Some(Some(Kind::File)) => return Ok(Metadata { is_file: true }),
            Some(Some(Kind::Dir)) => return Ok(Metadata { is_file: false }),
            Some(None) => return Err(not_found(path)),
            _ => {}
        }

        let inner = fs_err::metadata(path)?;

        Ok(Metadata {
            is_file: inner.is_file(),
        })
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        if let Some(canonical) = self.cache.lock().unwrap().canonical.get(path) {
            return Ok(canonical.clone());
        }

        // A listed file or directory that isn't a link is its parent's
        // canonical path plus its name, which saves a syscall per file.
        let canonical = match (path.parent(), path.file_name(), self.lookup(path, false)) {
            (Some(parent), Some(name), Some(Some(Kind::File | Kind::Dir))) => {
                join_canonical(&self.canonicalize(parent)?, name)
            }
            _ => dunce::canonicalize(path)?,
        };

        self.cache
            .lock()
            .unwrap()
            .canonical
            .insert(path.to_path_buf(), canonical.clone());

        Ok(canonical)
    }

    fn event_receiver(&self) -> crossbeam_channel::Receiver<VfsEvent> {
        self.watcher_receiver.clone()
    }

    fn watch(&self, path: &Path) -> io::Result<()> {
        let mut watches = self.watches.lock().unwrap();
        if watches.contains(path) || path.ancestors().any(|ancestor| watches.contains(ancestor)) {
            Ok(())
        } else {
            watches.insert(path.to_path_buf());
            self.watcher
                .lock()
                .unwrap()
                .watch(path, RecursiveMode::Recursive)
                .map_err(io::Error::other)
        }
    }

    fn unwatch(&mut self, path: &Path) -> io::Result<()> {
        self.watches.get_mut().unwrap().remove(path);
        self.watcher
            .get_mut()
            .unwrap()
            .unwatch(path)
            .map_err(io::Error::other)
    }
}
