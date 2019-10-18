//! Implements the VFS fetcher interface for the real filesystem using Rust's
//! std::fs interface and notify as the file watcher.

use std::{
    collections::HashSet,
    fs, io,
    path::{Path, PathBuf},
    sync::{mpsc, Mutex},
    time::Duration,
};

use crossbeam_channel::{unbounded, Receiver, Sender};
use jod_thread::JoinHandle;
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};

use super::{
    event::VfsEvent,
    fetcher::{FileType, VfsFetcher},
};

/// Workaround to disable the file watcher for processes that don't need it,
/// since notify appears hang on to mpsc Sender objects too long, causing Rojo
/// to deadlock on drop.
///
/// We can make constructing the watcher optional in order to hotfix rojo build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchMode {
    Enabled,
    Disabled,
}

pub struct RealFetcher {
    // Drop order is relevant here!
    //
    // `watcher` must be dropped before `_converter_thread` or else joining the
    // thread will cause a deadlock.
    watcher: Option<Mutex<RecommendedWatcher>>,

    /// Thread handle to convert notify's mpsc channel messages into
    /// crossbeam_channel messages.
    _converter_thread: JoinHandle<()>,

    /// The crossbeam receiver filled with events from the converter thread.
    receiver: Receiver<VfsEvent>,

    /// All of the paths that the fetcher is watching, tracked here because
    /// notify does not expose this information.
    watched_paths: Mutex<HashSet<PathBuf>>,
}

impl RealFetcher {
    pub fn new(watch_mode: WatchMode) -> RealFetcher {
        log::trace!("Starting RealFetcher with watch mode {:?}", watch_mode);

        let (notify_sender, notify_receiver) = mpsc::channel();
        let (sender, receiver) = unbounded();

        let handle = jod_thread::Builder::new()
            .name("notify message converter".to_owned())
            .spawn(move || {
                log::trace!("RealFetcher converter thread started");
                converter_thread(notify_receiver, sender);
                log::trace!("RealFetcher converter thread stopped");
            })
            .expect("Could not start message converter thread");

        // TODO: Investigate why notify hangs onto notify_sender too long,
        // causing our program to deadlock. Once this is fixed, watcher no
        // longer needs to be optional, but is still maybe useful?
        let watcher = match watch_mode {
            WatchMode::Enabled => {
                let watcher = notify::watcher(notify_sender, Duration::from_millis(300))
                    .expect("Couldn't start 'notify' file watcher");

                Some(Mutex::new(watcher))
            }
            WatchMode::Disabled => None,
        };

        RealFetcher {
            watcher,
            _converter_thread: handle,
            receiver,
            watched_paths: Mutex::new(HashSet::new()),
        }
    }
}

fn converter_thread(notify_receiver: mpsc::Receiver<DebouncedEvent>, sender: Sender<VfsEvent>) {
    use DebouncedEvent::*;

    for event in notify_receiver {
        log::trace!("Notify event: {:?}", event);

        match event {
            Create(path) => sender.send(VfsEvent::Created(path)).unwrap(),
            Write(path) => sender.send(VfsEvent::Modified(path)).unwrap(),
            Remove(path) => sender.send(VfsEvent::Removed(path)).unwrap(),
            Rename(from_path, to_path) => {
                sender.send(VfsEvent::Created(from_path)).unwrap();
                sender.send(VfsEvent::Removed(to_path)).unwrap();
            }
            Rescan => {
                log::warn!("Unhandled filesystem rescan event.");
                log::warn!(
                    "Please file an issue! Rojo may need to handle this case, but does not yet."
                );
            }
            Error(err, maybe_path) => {
                log::warn!("Unhandled filesystem error: {}", err);

                match maybe_path {
                    Some(path) => log::warn!("On path {}", path.display()),
                    None => log::warn!("No path was associated with this error."),
                }

                log::warn!(
                    "Rojo may need to handle this. If this happens again, please file an issue!"
                );
            }
            NoticeWrite(_) | NoticeRemove(_) | Chmod(_) => {}
        }
    }
}

impl VfsFetcher for RealFetcher {
    fn file_type(&self, path: &Path) -> io::Result<FileType> {
        let metadata = fs::metadata(path)?;

        if metadata.is_file() {
            Ok(FileType::File)
        } else {
            Ok(FileType::Directory)
        }
    }

    fn read_children(&self, path: &Path) -> io::Result<Vec<PathBuf>> {
        log::trace!("Reading directory {}", path.display());

        let mut result = Vec::new();

        let iter = fs::read_dir(path)?;

        for entry in iter {
            result.push(entry?.path());
        }

        Ok(result)
    }

    fn read_contents(&self, path: &Path) -> io::Result<Vec<u8>> {
        log::trace!("Reading file {}", path.display());

        fs::read(path)
    }

    fn create_directory(&self, path: &Path) -> io::Result<()> {
        log::trace!("Creating directory {}", path.display());

        fs::create_dir(path)
    }

    fn write_file(&self, path: &Path, contents: &[u8]) -> io::Result<()> {
        log::trace!("Writing path {}", path.display());

        fs::write(path, contents)
    }

    fn remove(&self, path: &Path) -> io::Result<()> {
        log::trace!("Removing path {}", path.display());

        let metadata = fs::metadata(path)?;

        if metadata.is_file() {
            fs::remove_file(path)
        } else {
            fs::remove_dir_all(path)
        }
    }

    fn watch(&self, path: &Path) {
        log::trace!("Watching path {}", path.display());

        if let Some(watcher_handle) = &self.watcher {
            let mut watcher = watcher_handle.lock().unwrap();

            match watcher.watch(path, RecursiveMode::NonRecursive) {
                Ok(_) => {
                    let mut watched_paths = self.watched_paths.lock().unwrap();
                    watched_paths.insert(path.to_path_buf());
                }
                Err(err) => {
                    log::warn!("Couldn't watch path {}: {:?}", path.display(), err);
                }
            }
        }
    }

    fn unwatch(&self, path: &Path) {
        log::trace!("Stopped watching path {}", path.display());

        if let Some(watcher_handle) = &self.watcher {
            let mut watcher = watcher_handle.lock().unwrap();

            // Remove the path from our watched paths regardless of the outcome
            // of notify's unwatch to ensure we drop old paths in the event of a
            // rename.
            let mut watched_paths = self.watched_paths.lock().unwrap();
            watched_paths.remove(path);

            if let Err(err) = watcher.unwatch(path) {
                log::warn!("Couldn't unwatch path {}: {:?}", path.display(), err);
            }
        }
    }

    fn receiver(&self) -> Receiver<VfsEvent> {
        self.receiver.clone()
    }

    fn watched_paths(&self) -> Vec<PathBuf> {
        let watched_paths = self.watched_paths.lock().unwrap();
        watched_paths.iter().cloned().collect()
    }
}
