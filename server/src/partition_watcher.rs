use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;
use std::thread;

use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher, watcher};

use partition::Partition;

pub struct PartitionWatcher {
    pub watcher: RecommendedWatcher,
    pub partition: Partition,
}

impl PartitionWatcher {
    pub fn start_new(partition: Partition, tx: Sender<(String, DebouncedEvent)>) -> PartitionWatcher {
        let (watch_tx, watch_rx) = channel();

        let mut watcher = watcher(watch_tx, Duration::from_millis(100)).unwrap();

        watcher.watch(&partition.path, RecursiveMode::Recursive).unwrap();

        let partition_name = partition.name.clone();
        thread::spawn(move || {
            loop {
                match watch_rx.recv() {
                    // TODO: Transform DebouncedEvent to some sort of FileChange object
                    Ok(event) => match tx.send((partition_name.clone(), event)) {
                        Ok(_) => {},
                        Err(_) => break,
                    },
                    Err(_) => break,
                };
            }
        });

        PartitionWatcher {
            partition,
            watcher,
        }
    }

    pub fn stop(self) {
    }
}
