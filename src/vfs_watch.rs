use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};

use vfs::Vfs;
use pathext::path_to_route;

pub struct VfsWatcher {
    vfs: Arc<Mutex<Vfs>>,
    watchers: Vec<RecommendedWatcher>,
}

impl VfsWatcher {
    pub fn new(vfs: Arc<Mutex<Vfs>>) -> VfsWatcher {
        VfsWatcher {
            vfs,
            watchers: Vec::new(),
        }
    }

    pub fn start(mut self) {
        {
            let outer_vfs = self.vfs.lock().unwrap();

            for (partition_name, root_path) in &outer_vfs.partitions {
                let (tx, rx) = mpsc::channel();
                let partition_name = partition_name.clone();
                let root_path = root_path.clone();

                let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_secs(1))
                    .expect("Unable to create watcher!");

                watcher
                    .watch(&root_path, RecursiveMode::Recursive)
                    .expect("Unable to watch path!");

                self.watchers.push(watcher);

                {
                    let vfs = self.vfs.clone();

                    thread::spawn(move || {
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
                                        println!("Failed to get route from {}", change_path.display());
                                    }
                                },
                                DebouncedEvent::Rename(ref from_change, ref to_change) => {
                                    if let Some(mut route) = path_to_route(&root_path, from_change) {
                                        route.insert(0, partition_name.clone());

                                        vfs.add_change(current_time, route);
                                    } else {
                                        println!("Failed to get route from {}", from_change.display());
                                    }

                                    if let Some(mut route) = path_to_route(&root_path, to_change) {
                                        route.insert(0, partition_name.clone());

                                        vfs.add_change(current_time, route);
                                    } else {
                                        println!("Failed to get route from {}", to_change.display());
                                    }
                                },
                                _ => {},
                            }
                        }
                    });
                }
            }
        }

        loop {}
    }
}
