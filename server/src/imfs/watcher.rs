use std::{
    sync::mpsc,
    path::Path,
    thread,
    time::Duration,
};

use crossbeam_channel::{Receiver, unbounded};
use notify::{Watcher, RecursiveMode, DebouncedEvent};

pub type ImfsEvent = DebouncedEvent;

pub trait ImfsWatcher {
    fn watch(&mut self, path: &Path);
    fn unwatch(&mut self, path: &Path);
    fn receiver(&mut self) -> Receiver<ImfsEvent>;
}

pub struct NoopWatcher;

impl ImfsWatcher for NoopWatcher {
    fn watch(&mut self, _path: &Path) {}
    fn unwatch(&mut self, _path: &Path) {}
    fn receiver(&mut self) -> Receiver<ImfsEvent> {
        crossbeam_channel::never()
    }
}

pub struct NotifyWatcher {
    inner: notify::RecommendedWatcher,
    _converter_thread: ScopedThread,
    receiver: Receiver<ImfsEvent>,
}

impl NotifyWatcher {
    pub fn new() -> NotifyWatcher {
        let (notify_sender, notify_receiver) = mpsc::channel();
        let (s, r) = unbounded();

        let watcher = notify::watcher(notify_sender, Duration::from_millis(300))
            .expect("Couldn't start 'notify' file watcher");

        let handle = ScopedThread::spawn("NotifyWatcher".to_owned(), move || {
            notify_receiver.into_iter()
                .for_each(|event| { s.send(event).unwrap() });
        });

        NotifyWatcher {
            inner: watcher,
            _converter_thread: handle,
            receiver: r,
        }
    }
}

impl ImfsWatcher for NotifyWatcher {
    fn watch(&mut self, path: &Path) {
        if let Err(err) = self.inner.watch(path, RecursiveMode::NonRecursive) {
            log::warn!("Couldn't watch path {}: {:?}", path.display(), err);
        }
    }

    fn unwatch(&mut self, path: &Path) {
        if let Err(err) = self.inner.unwatch(path) {
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