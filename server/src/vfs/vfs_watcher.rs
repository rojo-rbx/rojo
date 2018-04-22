use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};

use pathext::path_to_route;
use vfs::VfsSession;

/// An object that registers watchers on the real filesystem and relays those
/// changes to the virtual filesystem layer.
pub struct VfsWatcher {
    vfs: Arc<Mutex<VfsSession>>,
}

impl VfsWatcher {
    pub fn new(vfs: Arc<Mutex<VfsSession>>) -> VfsWatcher {
        VfsWatcher {
            vfs,
        }
    }

    fn start_watcher(
        vfs: Arc<Mutex<VfsSession>>,
        rx: mpsc::Receiver<DebouncedEvent>,
        partition_name: String,
        root_path: PathBuf,
    ) {
        loop {
            let event = rx.recv().unwrap();

            let mut vfs = vfs.lock().unwrap();
            let current_time = vfs.current_time();

            match event {
                DebouncedEvent::Write(ref change_path) |
                DebouncedEvent::Create(ref change_path) |
                DebouncedEvent::Remove(ref change_path) => {
                    if let Some(mut route) = path_to_route(&root_path, change_path) {
                        route.insert(0, partition_name.clone());

                        vfs.add_change(current_time, route);
                    } else {
                        eprintln!("Failed to get route from {}", change_path.display());
                    }
                },
                DebouncedEvent::Rename(ref from_change, ref to_change) => {
                    if let Some(mut route) = path_to_route(&root_path, from_change) {
                        route.insert(0, partition_name.clone());

                        vfs.add_change(current_time, route);
                    } else {
                        eprintln!("Failed to get route from {}", from_change.display());
                    }

                    if let Some(mut route) = path_to_route(&root_path, to_change) {
                        route.insert(0, partition_name.clone());

                        vfs.add_change(current_time, route);
                    } else {
                        eprintln!("Failed to get route from {}", to_change.display());
                    }
                },
                _ => {},
            }
        }
    }

    pub fn start(self) {
        let mut watchers = Vec::new();

        // Create an extra scope so that `vfs` gets dropped and unlocked
        {
            let vfs = self.vfs.lock().unwrap();

            for (ref partition_name, ref root_path) in vfs.get_partitions() {
                let (tx, rx) = mpsc::channel();

                let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_millis(200))
                    .expect("Unable to create watcher! This is a bug in Rojo.");

                match watcher.watch(&root_path, RecursiveMode::Recursive) {
                    Ok(_) => (),
                    Err(_) => {
                        panic!("Unable to watch partition {}, with path {}! Make sure that it's a file or directory.", partition_name, root_path.display());
                    },
                }

                watchers.push(watcher);

                {
                    let partition_name = partition_name.to_string();
                    let root_path = root_path.to_path_buf();
                    let vfs = self.vfs.clone();

                    thread::spawn(move || {
                        Self::start_watcher(vfs, rx, partition_name, root_path);
                    });
                }
            }
        }

        loop {
            thread::park();
        }
    }
}
