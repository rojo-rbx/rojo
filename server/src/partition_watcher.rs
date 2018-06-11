use std::sync::mpsc::{channel, Sender};
use std::time::Duration;
use std::thread;

use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher, watcher};

use partition::Partition;
use vfs_session::FileChange;
use file_route::FileRoute;

const WATCH_TIMEOUT_MS: u64 = 100;

pub struct PartitionWatcher {
    pub watcher: RecommendedWatcher,
}

impl PartitionWatcher {
    pub fn start_new(partition: Partition, tx: Sender<FileChange>) -> PartitionWatcher {
        let (watch_tx, watch_rx) = channel();

        let mut watcher = watcher(watch_tx, Duration::from_millis(WATCH_TIMEOUT_MS)).unwrap();

        watcher.watch(&partition.path, RecursiveMode::Recursive).unwrap();

        thread::spawn(move || {
            loop {
                match watch_rx.recv() {
                    Ok(event) => {
                        let file_change = match event {
                            DebouncedEvent::Create(path) => {
                                let route = FileRoute::from_path(&path, &partition).unwrap();
                                FileChange::Created(route)
                            },
                            DebouncedEvent::Write(path) => {
                                let route = FileRoute::from_path(&path, &partition).unwrap();
                                FileChange::Updated(route)
                            },
                            DebouncedEvent::Remove(path) => {
                                let route = FileRoute::from_path(&path, &partition).unwrap();
                                FileChange::Deleted(route)
                            },
                            DebouncedEvent::Rename(from_path, to_path) => {
                                let from_route = FileRoute::from_path(&from_path, &partition).unwrap();
                                let to_route = FileRoute::from_path(&to_path, &partition).unwrap();
                                FileChange::Moved(from_route, to_route)
                            },
                            _ => continue,
                        };

                        match tx.send(file_change) {
                            Ok(_) => {},
                            Err(_) => break,
                        }
                    },
                    Err(_) => break,
                };
            }
        });

        PartitionWatcher {
            watcher,
        }
    }

    pub fn stop(self) {
    }
}
