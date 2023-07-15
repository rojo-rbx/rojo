use std::io;
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crate::{DirEntry, Metadata, ReadDir, VfsBackend, VfsEvent};
use crossbeam_channel::Receiver;
use notify::{
    event::{MetadataKind, ModifyKind, RenameMode},
    EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use notify_debouncer_full::{new_debouncer, Debouncer, FileIdMap};

/// `VfsBackend` that uses `std::fs` and the `notify` crate.
pub struct StdBackend {
    watcher: Debouncer<RecommendedWatcher, FileIdMap>,
    watcher_receiver: Receiver<VfsEvent>,
}

impl StdBackend {
    pub fn new() -> StdBackend {
        let (notify_tx, notify_rx) = mpsc::channel();
        let watcher = new_debouncer(Duration::from_millis(50), None, notify_tx).unwrap();

        let (tx, rx) = crossbeam_channel::unbounded();

        thread::spawn(move || {
            for notification in notify_rx {
                match notification {
                    Ok(events) => {
                        for event in events {
                            if event.need_rescan() {
                                // In this case, we need to refresh our file.
                                // So, we can ignore the actual event (anyhow, should always be an "Other" event after auditing the code)!
                                let path = event.paths[0].clone();

                                match fs_err::OpenOptions::new().open(&path) {
                                    // The file still exists, we'll just say that it changed.
                                    Ok(_) => tx.send(VfsEvent::Write(path))?,

                                    // It is inaccessible, it doesn't exist to us.
                                    Err(_) => tx.send(VfsEvent::Remove(path))?,
                                };

                                return Ok(());
                            }

                            match event.kind {
                                // Vfs does not care for how the file was made.
                                EventKind::Create(_) => {
                                    tx.send(VfsEvent::Create(event.paths[0].clone()))?
                                }

                                // Vfs doesn't care for how the file was removed.
                                EventKind::Remove(_) => {
                                    tx.send(VfsEvent::Remove(event.paths[0].clone()))?
                                }

                                EventKind::Modify(modify_kind) => match modify_kind {
                                    // We don't care for how the data was changed, it was some write.
                                    ModifyKind::Data(_) => {
                                        tx.send(VfsEvent::Write(event.paths[0].clone()))?
                                    }

                                    // There's several ways a rename event can be represented.
                                    ModifyKind::Name(rename_mode) => match rename_mode {
                                        // We get the original and the new name at the same time.
                                        RenameMode::Both => {
                                            tx.send(VfsEvent::Remove(event.paths[0].clone()))?;
                                            tx.send(VfsEvent::Create(event.paths[1].clone()))?;
                                        }

                                        // We could also get with two separate events.
                                        // Start with the original name...
                                        RenameMode::From => {
                                            tx.send(VfsEvent::Remove(event.paths[0].clone()))?;
                                        }

                                        // And end with the new name.
                                        RenameMode::To => {
                                            tx.send(VfsEvent::Create(event.paths[0].clone()))?;
                                        }

                                        // After auditing the code, this event will be fired if the original file was renamed with fsevent and kqueue.
                                        // However, no way to actually say what it was renamed to...
                                        RenameMode::Any => {
                                            tx.send(VfsEvent::Remove(event.paths[0].clone()))?;
                                        }

                                        // After auditing the code, this event will never be fired.
                                        RenameMode::Other => panic!(
                                            r#"EventKind(ModifyKind::Name(RenameMode::Other))) was impossibly issued!
                                            Log an issue at memofs (rbx-rojo) with the following information:
                                            {:#?}"#,
                                            event
                                        ),
                                    },

                                    // Vfs does not care for any metadata changes.
                                    // However, we do care if a file becomes inaccessible or not (turns out, notify will not fire a Create/Remove event).
                                    ModifyKind::Metadata(kind) => {
                                        match kind {
                                            // It doesn't matter if these metadata changes.
                                            MetadataKind::Extended
                                            | MetadataKind::AccessTime
                                            | MetadataKind::WriteTime => {}

                                            // However, these metadata changes could make a file/directory inaccessible or accessible.
                                            // TODO: Fix so that we emit create/removals. For now, the behavior prior was to ignore such changes.
                                            MetadataKind::Any
                                            | MetadataKind::Ownership
                                            | MetadataKind::Permissions
                                            | MetadataKind::Other => {}
                                        }
                                    }

                                    // After auditing the code, the only way for Modify::Any to be triggered is if the backend is...
                                    // kqueue, then the number of links to the given path changed. We don't really care on that case.
                                    // windows, just vaguely that a file was changed.
                                    ModifyKind::Any => {
                                        if cfg!(windows) {
                                            tx.send(VfsEvent::Write(event.paths[0].clone()))?;
                                        }
                                    }

                                    // After auditing notify, this will never be fired.
                                    ModifyKind::Other => panic!(
                                        r#"EventKind::Modify(ModifyKind::Other()) was impossibly issued!
                                        Log an issue at memofs (rbx-rojo) with the following information:
                                        {:#?}"#,
                                        event
                                    ),
                                },

                                // We don't need to send an event if a file is read, opened, or closed
                                EventKind::Access(_) => {}

                                // After auditing notify, this can only be fired by fsevent, but that's only imprecise mode.
                                // Imprecise mode is never enabled in notify (thank goodness).
                                EventKind::Any => panic!(
                                    r#"EventKind::Any() was impossibly issued!
                                    Log an issue with the following information:
                                    {:#?}"#,
                                    event
                                ),

                                // After auditing notify, the only way to get here is if there was a unknown event emitted by kqueue.
                                // This is despite fsevent and inotify being able to emit this event
                                // (this is caught earlier since they both require rescanning).
                                // Might as well panic here and let the future us figure it out, since it seems extremely niche.
                                EventKind::Other => panic!(
                                    r#"EventKind::Other() was impossibly issued!
                                    Log an issue with the following information:
                                    {:#?}"#,
                                    event
                                ),
                            }
                        }
                    }

                    Err(errors) => {
                        for error in errors {
                            match error.kind {
                                notify::ErrorKind::Generic(generic) => panic!("Internal notify error (memofs): {}", generic),
                                notify::ErrorKind::InvalidConfig(config) => panic!("Internal memofs error: Invalid configuration for the watcher. How did we get here?\n{:?}", config),
                                notify::ErrorKind::MaxFilesWatch => panic!("Internal notify error (memofs): The maximum amount of files that can be kept track of has been reached!"),

                                notify::ErrorKind::Io(err) => todo!("What happens when IO errors like this: {}", err),
                                notify::ErrorKind::PathNotFound => {
                                    let path = error.paths[0].clone();
                                    println!("memofs warning: path {} was not found!", path.display());

                                    // This seems like a reasonable default.
                                    tx.send(VfsEvent::Remove(path))?
                                },
                                notify::ErrorKind::WatchNotFound => {
                                    let path = error.paths[0].clone();
                                    println!("memofs warning: watch for path {} was not found! Internal error of notify?", path.display());

                                    // TODO: I think it's probably best to remove the object from our side and try rewatching.
                                    // However, we have not a way to see if was a new object we're watching or an old object.
                                },
                            }
                        }
                    }
                }
            }

            Result::<(), crossbeam_channel::SendError<VfsEvent>>::Ok(())
        });

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
            .watcher()
            .watch(path, RecursiveMode::NonRecursive)
            .map_err(|inner| io::Error::new(io::ErrorKind::Other, inner))
    }

    fn unwatch(&mut self, path: &Path) -> io::Result<()> {
        self.watcher
            .watcher()
            .unwatch(path)
            .map_err(|inner| io::Error::new(io::ErrorKind::Other, inner))
    }
}

impl Default for StdBackend {
    fn default() -> Self {
        Self::new()
    }
}
