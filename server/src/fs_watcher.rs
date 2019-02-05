use std::{
    sync::{mpsc, Arc, Mutex},
    time::Duration,
    path::Path,
    ops::Deref,
    thread,
};

use log::{warn, trace};
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

/// Watches for changes on the filesystem and links together the in-memory
/// filesystem and in-memory Roblox tree.
pub struct FsWatcher {
    watcher: RecommendedWatcher,
}

impl FsWatcher {
    /// Start a new FS watcher, watching all of the roots currently attached to
    /// the given Imfs.
    ///
    /// `rbx_session` is optional to make testing easier. If it isn't `None`,
    /// events will be passed to it after they're given to the Imfs.
    pub fn start(imfs: Arc<Mutex<Imfs>>, rbx_session: Option<Arc<Mutex<RbxSession>>>) -> FsWatcher {
        let (watch_tx, watch_rx) = mpsc::channel();

        let mut watcher = notify::watcher(watch_tx, WATCH_TIMEOUT)
            .expect("Could not create filesystem watcher");

        {
            let imfs = imfs.lock().unwrap();

            for root_path in imfs.get_roots() {
                trace!("Watching path {}", root_path.display());
                watcher.watch(root_path, RecursiveMode::Recursive)
                    .expect("Could not watch directory");
            }
        }

        {
            let imfs = Arc::clone(&imfs);
            let rbx_session = rbx_session.as_ref().map(Arc::clone);

            thread::spawn(move || {
                trace!("Watcher thread started");
                while let Ok(event) = watch_rx.recv() {
                    // handle_fs_event expects an Option<&Mutex<T>>, but we have
                    // an Option<Arc<Mutex<T>>>, so we coerce with Deref.
                    let session_ref = rbx_session.as_ref().map(Deref::deref);

                    handle_fs_event(&imfs, session_ref, event);
                }
                trace!("Watcher thread stopped");
            });
        }

        FsWatcher {
            watcher,
        }
    }

    pub fn stop_watching_path(&mut self, path: &Path) {
        match self.watcher.unwatch(path) {
            Ok(_) => {},
            Err(e) => {
                warn!("Could not unwatch path {}: {}", path.display(), e);
            },
        }
    }
}

fn handle_fs_event(imfs: &Mutex<Imfs>, rbx_session: Option<&Mutex<RbxSession>>, event: DebouncedEvent) {
    match event {
        DebouncedEvent::Create(path) => {
            trace!("Path created: {}", path.display());

            {
                let mut imfs = imfs.lock().unwrap();
                imfs.path_created(&path).unwrap();
            }

            if let Some(rbx_session) = rbx_session {
                let mut rbx_session = rbx_session.lock().unwrap();
                rbx_session.path_created(&path);
            }
        },
        DebouncedEvent::Write(path) => {
            trace!("Path created: {}", path.display());

            {
                let mut imfs = imfs.lock().unwrap();
                imfs.path_updated(&path).unwrap();
            }

            if let Some(rbx_session) = rbx_session {
                let mut rbx_session = rbx_session.lock().unwrap();
                rbx_session.path_updated(&path);
            }
        },
        DebouncedEvent::Remove(path) => {
            trace!("Path removed: {}", path.display());

            {
                let mut imfs = imfs.lock().unwrap();
                imfs.path_removed(&path).unwrap();
            }

            if let Some(rbx_session) = rbx_session {
                let mut rbx_session = rbx_session.lock().unwrap();
                rbx_session.path_removed(&path);
            }
        },
        DebouncedEvent::Rename(from_path, to_path) => {
            trace!("Path renamed: {} to {}", from_path.display(), to_path.display());

            {
                let mut imfs = imfs.lock().unwrap();
                imfs.path_moved(&from_path, &to_path).unwrap();
            }

            if let Some(rbx_session) = rbx_session {
                let mut rbx_session = rbx_session.lock().unwrap();
                rbx_session.path_renamed(&from_path, &to_path);
            }
        },
        other => {
            trace!("Unhandled FS event: {:?}", other);
        },
    }
}