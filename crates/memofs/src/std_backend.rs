use std::io;
use std::path::Path;

use crossbeam_channel::Receiver;

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

#[cfg(target_os = "macos")]
use notify::{Config, PollWatcher};

#[cfg(target_os = "macos")]
use std::time::Duration;

use crate::{DirEntry, Metadata, ReadDir, VfsBackend, VfsEvent};

/// `VfsBackend` that uses `std::fs` and the `notify` crate.
pub struct StdBackend {
    #[cfg(target_os = "macos")]
    watcher: PollWatcher,

    #[cfg(not(target_os = "macos"))]
    watcher: RecommendedWatcher,

    watcher_receiver: Receiver<VfsEvent>,
}

impl StdBackend {
    pub fn new() -> StdBackend {
        let (tx, rx) = crossbeam_channel::unbounded();

        let event_handler = move |res: Result<Event, _>| match res {
            Ok(event) => match event.kind {
                EventKind::Create(_) => {
                    for path in event.paths {
                        let _ = tx.send(VfsEvent::Create(path));
                    }
                }
                EventKind::Modify(_) => {
                    for path in event.paths {
                        let _ = tx.send(VfsEvent::Write(path));
                    }
                }
                EventKind::Remove(_) => {
                    for path in event.paths {
                        let _ = tx.send(VfsEvent::Remove(path));
                    }
                }
                _ => {}
            },
            Err(e) => println!("watch error: {:?}", e),
        };

        #[cfg(not(target_os = "macos"))]
        let watcher = notify::recommended_watcher(event_handler).unwrap();

        #[cfg(target_os = "macos")]
        let watcher = PollWatcher::new(
            event_handler,
            Config::default().with_poll_interval(Duration::from_millis(200)),
        )
        .unwrap();

        Self {
            watcher,
            watcher_receiver: rx,
        }
    }
}

impl VfsBackend for StdBackend {
    fn read(&mut self, path: &Path) -> io::Result<Vec<u8>> {
        fs_err::read(path)
    }

    fn write(&mut self, path: &Path, data: &[u8]) -> io::Result<()> {
        fs_err::write(path, data)
    }

    fn read_dir(&mut self, path: &Path) -> io::Result<ReadDir> {
        let entries: Result<Vec<_>, _> = fs_err::read_dir(path)?.collect();
        let mut entries = entries?;

        entries.sort_by_cached_key(|entry| entry.file_name());

        let inner = entries
            .into_iter()
            .map(|entry| Ok(DirEntry { path: entry.path() }));

        Ok(ReadDir {
            inner: Box::new(inner),
        })
    }

    fn remove_file(&mut self, path: &Path) -> io::Result<()> {
        fs_err::remove_file(path)
    }

    fn remove_dir_all(&mut self, path: &Path) -> io::Result<()> {
        fs_err::remove_dir_all(path)
    }

    fn metadata(&mut self, path: &Path) -> io::Result<Metadata> {
        let inner = fs_err::metadata(path)?;

        Ok(Metadata {
            is_file: inner.is_file(),
        })
    }

    fn event_receiver(&self) -> crossbeam_channel::Receiver<VfsEvent> {
        self.watcher_receiver.clone()
    }

    fn watch(&mut self, path: &Path) -> io::Result<()> {
        self.watcher
            .watch(path, RecursiveMode::NonRecursive)
            .map_err(|inner| io::Error::new(io::ErrorKind::Other, inner))
    }

    fn unwatch(&mut self, path: &Path) -> io::Result<()> {
        self.watcher
            .unwatch(path)
            .map_err(|inner| io::Error::new(io::ErrorKind::Other, inner))
    }
}

impl Default for StdBackend {
    fn default() -> Self {
        Self::new()
    }
}
