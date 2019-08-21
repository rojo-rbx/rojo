//! Implements the IMFS fetcher interface for the real filesystem using Rust's
//! std::fs interface and notify as the file watcher.

use std::{
    fs,
    io,
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
    time::Duration,
};

use crossbeam_channel::{Receiver, unbounded};
use notify::{RecursiveMode, RecommendedWatcher, Watcher};

use super::fetcher::{ImfsFetcher, FileType, ImfsEvent};

pub struct RealFetcher {
    watcher: RecommendedWatcher,

    /// Thread handle to convert notify's mpsc channel messages into
    /// crossbeam_channel messages.
    _converter_thread: ScopedThread,
    receiver: Receiver<ImfsEvent>,
}

impl RealFetcher {
    pub fn new() -> RealFetcher {
        let (notify_sender, notify_receiver) = mpsc::channel();
        let (s, r) = unbounded();

        let watcher = notify::watcher(notify_sender, Duration::from_millis(300))
            .expect("Couldn't start 'notify' file watcher");

        let handle = ScopedThread::spawn("RealFetcher".to_owned(), move || {
            notify_receiver.into_iter()
                .for_each(|event| { s.send(event).unwrap() });
        });

        RealFetcher {
            watcher,
            _converter_thread: handle,
            receiver: r,
        }
    }
}

impl ImfsFetcher for RealFetcher {
    fn file_type(&mut self, path: &Path) -> io::Result<FileType> {
        let metadata = fs::metadata(path)?;

        if metadata.is_file() {
            Ok(FileType::File)
        } else {
            Ok(FileType::Directory)
        }
    }

    fn read_children(&mut self, path: &Path) -> io::Result<Vec<PathBuf>> {
        let mut result = Vec::new();

        let iter = fs::read_dir(path)?;

        for entry in iter {
            result.push(entry?.path());
        }

        Ok(result)
    }

    fn read_contents(&mut self, path: &Path) -> io::Result<Vec<u8>> {
        fs::read(path)
    }

    fn create_directory(&mut self, path: &Path) -> io::Result<()> {
        fs::create_dir(path)
    }

    fn write_file(&mut self, path: &Path, contents: &[u8]) -> io::Result<()> {
        fs::write(path, contents)
    }

    fn remove(&mut self, path: &Path) -> io::Result<()> {
        let metadata = fs::metadata(path)?;

        if metadata.is_file() {
            fs::remove_file(path)
        } else {
            fs::remove_dir_all(path)
        }
    }

    fn watch(&mut self, path: &Path) {
        if let Err(err) = self.watcher.watch(path, RecursiveMode::NonRecursive) {
            log::warn!("Couldn't watch path {}: {:?}", path.display(), err);
        }
    }

    fn unwatch(&mut self, path: &Path) {
        if let Err(err) = self.watcher.unwatch(path) {
            log::warn!("Couldn't unwatch path {}: {:?}", path.display(), err);
        }
    }

    fn receiver(&mut self) -> Receiver<ImfsEvent> {
        self.receiver.clone()
    }
}

// Join-on-drop thread implementation from ra_vfs, maybe this should be a crate?
struct ScopedThread(Option<thread::JoinHandle<()>>);

impl ScopedThread {
    fn spawn(name: String, f: impl FnOnce() + Send + 'static) -> ScopedThread {
        let handle = thread::Builder::new().name(name).spawn(f).unwrap();

        ScopedThread(Some(handle))
    }
}

impl Drop for ScopedThread {
    fn drop(&mut self) {
        let res = self.0.take().unwrap().join();

        if !thread::panicking() {
            res.unwrap();
        }
    }
}