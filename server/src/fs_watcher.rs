use std::{
    sync::{mpsc, Arc, Mutex},
    time::Duration,
    thread,
};

use notify::{
    self,
    DebouncedEvent,
    RecommendedWatcher,
    RecursiveMode,
    Watcher,
};

use crate::{
    vfs::Vfs,
    rbx_session::RbxSession,
};

const WATCH_TIMEOUT: Duration = Duration::from_millis(100);

fn handle_event(vfs: &Mutex<Vfs>, rbx_session: &Mutex<RbxSession>, event: DebouncedEvent) {
    match event {
        DebouncedEvent::Create(path) => {
            {
                let mut vfs = vfs.lock().unwrap();
                vfs.path_created(&path).unwrap();
            }

            {
                let mut rbx_session = rbx_session.lock().unwrap();
                rbx_session.path_created(&path);
            }
        },
        DebouncedEvent::Write(path) => {
            {
                let mut vfs = vfs.lock().unwrap();
                vfs.path_updated(&path).unwrap();
            }

            {
                let mut rbx_session = rbx_session.lock().unwrap();
                rbx_session.path_updated(&path);
            }
        },
        DebouncedEvent::Remove(path) => {
            {
                let mut vfs = vfs.lock().unwrap();
                vfs.path_removed(&path).unwrap();
            }

            {
                let mut rbx_session = rbx_session.lock().unwrap();
                rbx_session.path_removed(&path);
            }
        },
        DebouncedEvent::Rename(from_path, to_path) => {
            {
                let mut vfs = vfs.lock().unwrap();
                vfs.path_moved(&from_path, &to_path).unwrap();
            }

            {
                let mut rbx_session = rbx_session.lock().unwrap();
                rbx_session.path_renamed(&from_path, &to_path);
            }
        },
        _ => {},
    }
}

/// Watches for changes on the filesystem and links together the in-memory
/// filesystem and in-memory Roblox tree.
pub struct FsWatcher {
    #[allow(unused)]
    watchers: Vec<RecommendedWatcher>,
}

impl FsWatcher {
    pub fn start(vfs: Arc<Mutex<Vfs>>, rbx_session: Arc<Mutex<RbxSession>>) -> FsWatcher {
        let mut watchers = Vec::new();

        {
            let vfs_temp = vfs.lock().unwrap();

            for root_path in vfs_temp.get_roots() {
                let (watch_tx, watch_rx) = mpsc::channel();

                let mut watcher = notify::watcher(watch_tx, WATCH_TIMEOUT)
                    .expect("Could not create `notify` watcher");

                watcher.watch(root_path, RecursiveMode::Recursive)
                    .expect("Could not watch directory");

                watchers.push(watcher);

                let vfs = Arc::clone(&vfs);
                let rbx_session = Arc::clone(&rbx_session);
                let root_path = root_path.clone();

                thread::spawn(move || {
                    info!("Watcher thread ({}) started", root_path.display());
                    loop {
                        match watch_rx.recv() {
                            Ok(event) => handle_event(&vfs, &rbx_session, event),
                            Err(_) => break,
                        };
                    }
                    info!("Watcher thread ({}) stopped", root_path.display());
                });
            }
        }

        FsWatcher {
            watchers,
        }
    }
}