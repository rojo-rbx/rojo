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
    imfs::Imfs,
    rbx_session::RbxSession,
};

const WATCH_TIMEOUT: Duration = Duration::from_millis(100);

fn handle_event(imfs: &Mutex<Imfs>, rbx_session: &Mutex<RbxSession>, event: DebouncedEvent) {
    match event {
        DebouncedEvent::Create(path) => {
            {
                let mut imfs = imfs.lock().unwrap();
                imfs.path_created(&path).unwrap();
            }

            {
                let mut rbx_session = rbx_session.lock().unwrap();
                rbx_session.path_created(&path);
            }
        },
        DebouncedEvent::Write(path) => {
            {
                let mut imfs = imfs.lock().unwrap();
                imfs.path_updated(&path).unwrap();
            }

            {
                let mut rbx_session = rbx_session.lock().unwrap();
                rbx_session.path_updated(&path);
            }
        },
        DebouncedEvent::Remove(path) => {
            {
                let mut imfs = imfs.lock().unwrap();
                imfs.path_removed(&path).unwrap();
            }

            {
                let mut rbx_session = rbx_session.lock().unwrap();
                rbx_session.path_removed(&path);
            }
        },
        DebouncedEvent::Rename(from_path, to_path) => {
            {
                let mut imfs = imfs.lock().unwrap();
                imfs.path_moved(&from_path, &to_path).unwrap();
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
    pub fn start(imfs: Arc<Mutex<Imfs>>, rbx_session: Arc<Mutex<RbxSession>>) -> FsWatcher {
        let mut watchers = Vec::new();

        {
            let imfs_temp = imfs.lock().unwrap();

            for root_path in imfs_temp.get_roots() {
                let (watch_tx, watch_rx) = mpsc::channel();

                let mut watcher = notify::watcher(watch_tx, WATCH_TIMEOUT)
                    .expect("Could not create `notify` watcher");

                watcher.watch(root_path, RecursiveMode::Recursive)
                    .expect("Could not watch directory");

                watchers.push(watcher);

                let imfs = Arc::clone(&imfs);
                let rbx_session = Arc::clone(&rbx_session);
                let root_path = root_path.clone();

                thread::spawn(move || {
                    info!("Watcher thread ({}) started", root_path.display());
                    while let Ok(event) = watch_rx.recv() {
                        handle_event(&imfs, &rbx_session, event);
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