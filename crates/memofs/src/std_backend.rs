#[cfg(windows)]
use std::collections::HashMap;
use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
#[cfg(windows)]
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crossbeam_channel::Receiver;
use notify::{watcher, DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};

use crate::{DirEntry, Metadata, ReadDir, VfsBackend, VfsEvent};

/// `VfsBackend` that uses `std::fs` and the `notify` crate.
pub struct StdBackend {
    watcher: RecommendedWatcher,
    watcher_receiver: Receiver<VfsEvent>,
    #[cfg(not(windows))]
    watches: HashSet<PathBuf>,
    #[cfg(windows)]
    watches: WindowsWatchState,
    #[cfg(windows)]
    symlink_index: Arc<Mutex<WindowsSymlinkIndex>>,
}

#[cfg(windows)]
struct WindowsWatchRegistration {
    /// The target the logical path currently resolves to.
    physical_path: Option<PathBuf>,
    /// The last target of a still-present, but currently dangling, symlink.
    ///
    /// Keeping this watch alive lets an atomic remove/create sequence restore
    /// the same target without requiring a server restart.
    last_physical_path: Option<PathBuf>,
    /// Whether this logical path has ever been observed as a symlink.
    ///
    /// This remains true while the link is missing so its parent stays watched
    /// for recreation or replacement.
    is_symlink: bool,
    /// Whether `symlink_metadata` can currently see the link itself.
    is_present_symlink: bool,
}

#[cfg(windows)]
#[derive(Default)]
struct WindowsWatchState {
    /// Logical requests are retained even when another request's native watch
    /// covers them, because symlink targets can fall outside that ancestor.
    registrations: HashMap<PathBuf, WindowsWatchRegistration>,
    /// The minimal set actually registered with `notify`.
    native_watches: HashSet<PathBuf>,
}

#[cfg(windows)]
#[derive(Default)]
struct WindowsSymlinkIndex {
    /// Used by the watcher thread to recognize deletion of a known alias after
    /// the path can no longer be queried.
    logical_paths: HashSet<PathBuf>,
    /// Fans physical target lifecycle events back out to every logical alias.
    physical_to_logical: HashMap<PathBuf, HashSet<PathBuf>>,
}

#[cfg(windows)]
#[derive(Clone, Copy)]
enum WindowsEventKind {
    Create,
    Write,
    Remove,
}

impl StdBackend {
    pub fn new() -> io::Result<StdBackend> {
        let (notify_tx, notify_rx) = mpsc::channel();
        let watcher = watcher(notify_tx, Duration::from_millis(50)).map_err(io::Error::other)?;

        let (tx, rx) = crossbeam_channel::unbounded();
        #[cfg(windows)]
        let symlink_index = Arc::new(Mutex::new(WindowsSymlinkIndex::default()));
        #[cfg(windows)]
        let thread_symlink_index = Arc::clone(&symlink_index);

        thread::spawn(move || {
            for event in notify_rx {
                match event {
                    DebouncedEvent::Create(path) => {
                        #[cfg(windows)]
                        {
                            send_windows_event(
                                &tx,
                                &thread_symlink_index,
                                path,
                                WindowsEventKind::Create,
                            )?;
                        }
                        #[cfg(not(windows))]
                        {
                            tx.send(VfsEvent::Create(path))?;
                        }
                    }
                    DebouncedEvent::Write(path) => {
                        #[cfg(windows)]
                        {
                            send_windows_event(
                                &tx,
                                &thread_symlink_index,
                                path,
                                WindowsEventKind::Write,
                            )?;
                        }
                        #[cfg(not(windows))]
                        {
                            tx.send(VfsEvent::Write(path))?;
                        }
                    }
                    DebouncedEvent::Remove(path) => {
                        #[cfg(windows)]
                        {
                            send_windows_event(
                                &tx,
                                &thread_symlink_index,
                                path,
                                WindowsEventKind::Remove,
                            )?;
                        }
                        #[cfg(not(windows))]
                        {
                            tx.send(VfsEvent::Remove(path))?;
                        }
                    }
                    DebouncedEvent::Rename(from, to) => {
                        #[cfg(windows)]
                        {
                            send_windows_event(
                                &tx,
                                &thread_symlink_index,
                                from,
                                WindowsEventKind::Remove,
                            )?;
                            send_windows_event(
                                &tx,
                                &thread_symlink_index,
                                to,
                                WindowsEventKind::Create,
                            )?;
                        }
                        #[cfg(not(windows))]
                        {
                            tx.send(VfsEvent::Remove(from))?;
                            tx.send(VfsEvent::Create(to))?;
                        }
                    }
                    _ => {}
                }
            }

            Result::<(), crossbeam_channel::SendError<VfsEvent>>::Ok(())
        });

        Ok(Self {
            watcher,
            watcher_receiver: rx,
            #[cfg(not(windows))]
            watches: HashSet::new(),
            #[cfg(windows)]
            watches: WindowsWatchState::default(),
            #[cfg(windows)]
            symlink_index,
        })
    }

    #[cfg(windows)]
    fn watch_windows(&mut self, path: &Path) -> io::Result<()> {
        let logical_path = logical_path(path)?;
        let is_symlink = std::fs::symlink_metadata(&logical_path)
            .map(|metadata| metadata.file_type().is_symlink())
            .unwrap_or(false);
        let physical_path = match dunce::canonicalize(&logical_path) {
            Ok(path) => Some(path),
            Err(err) if err.kind() == io::ErrorKind::NotFound => None,
            Err(err) => return Err(err),
        };

        // A logical ancestor only covers this request if its physical target
        // is also an ancestor. This is the distinction the old HashSet missed:
        // a source root contains a symlink lexically, but not its external
        // target physically.
        if !is_symlink
            && physical_path.as_ref().is_some_and(|physical_path| {
                self.watches
                    .registrations
                    .iter()
                    .any(|(existing_logical, existing)| {
                        logical_path.starts_with(existing_logical)
                            && existing
                                .physical_path
                                .as_ref()
                                .is_some_and(|existing_physical| {
                                    physical_path.starts_with(existing_physical)
                                })
                    })
            })
        {
            return Ok(());
        }

        self.watches.registrations.insert(
            logical_path.clone(),
            WindowsWatchRegistration {
                last_physical_path: physical_path.clone(),
                physical_path,
                is_symlink,
                is_present_symlink: is_symlink,
            },
        );

        self.rebuild_symlink_index();
        self.sync_windows_watches()
    }

    #[cfg(windows)]
    fn unwatch_windows(&mut self, path: &Path) -> io::Result<()> {
        let logical_path = logical_path(path)?;
        if self.watches.registrations.remove(&logical_path).is_some() {
            self.rebuild_symlink_index();
            self.sync_windows_watches()?;
        }

        Ok(())
    }

    #[cfg(windows)]
    fn refresh_symlink_windows(&mut self, path: &Path) -> io::Result<()> {
        let logical_path = logical_path(path)?;
        let currently_symlink = std::fs::symlink_metadata(&logical_path)
            .map(|metadata| metadata.file_type().is_symlink())
            .unwrap_or(false);
        let physical_path = match dunce::canonicalize(&logical_path) {
            Ok(path) => Some(path),
            Err(err) if err.kind() == io::ErrorKind::NotFound => None,
            Err(err) => return Err(err),
        };

        match self.watches.registrations.get_mut(&logical_path) {
            Some(registration) => {
                if let Some(physical_path) = &physical_path {
                    registration.last_physical_path = Some(physical_path.clone());
                } else if !currently_symlink {
                    registration.last_physical_path = None;
                }
                registration.physical_path = physical_path;
                registration.is_symlink |= currently_symlink;
                registration.is_present_symlink = currently_symlink;
            }
            None if currently_symlink => {
                self.watches.registrations.insert(
                    logical_path.clone(),
                    WindowsWatchRegistration {
                        last_physical_path: physical_path.clone(),
                        physical_path,
                        is_symlink: true,
                        is_present_symlink: true,
                    },
                );
            }
            None => return Ok(()),
        }

        self.rebuild_symlink_index();
        self.sync_windows_watches()
    }

    #[cfg(windows)]
    fn rebuild_symlink_index(&self) {
        let mut index = self.symlink_index.lock().unwrap();
        index.logical_paths.clear();
        index.physical_to_logical.clear();

        for (logical_path, registration) in &self.watches.registrations {
            if !registration.is_symlink {
                continue;
            }

            index.logical_paths.insert(logical_path.clone());
            if let Some(physical_path) = registration
                .physical_path
                .as_ref()
                .or(registration.last_physical_path.as_ref())
            {
                index
                    .physical_to_logical
                    .entry(physical_path.clone())
                    .or_default()
                    .insert(logical_path.clone());
            }
        }
    }

    #[cfg(windows)]
    fn sync_windows_watches(&mut self) -> io::Result<()> {
        // Re-deriving the minimal set from logical ownership makes shared
        // targets behave like reference-counted watches without maintaining a
        // second, error-prone counter.
        let desired = minimal_watch_targets(self.watches.registrations.iter().flat_map(
            |(logical_path, registration)| {
                let physical_path = registration.physical_path.as_ref().or_else(|| {
                    registration
                        .is_present_symlink
                        .then_some(registration.last_physical_path.as_ref())
                        .flatten()
                });

                physical_path.cloned().into_iter().chain(
                    registration
                        .is_symlink
                        .then(|| logical_path.parent().map(Path::to_path_buf))
                        .flatten(),
                )
            },
        ));

        let additions: Vec<_> = desired
            .difference(&self.watches.native_watches)
            .cloned()
            .collect();
        for path in additions {
            self.watcher
                .watch(&path, RecursiveMode::Recursive)
                .map_err(io::Error::other)?;
            self.watches.native_watches.insert(path);
        }

        let removals: Vec<_> = self
            .watches
            .native_watches
            .difference(&desired)
            .cloned()
            .collect();
        for path in removals {
            // A deleted or replaced symlink target may already have caused
            // notify to discard the native watch.
            let _ = self.watcher.unwatch(&path);
            self.watches.native_watches.remove(&path);
        }

        Ok(())
    }
}

#[cfg(windows)]
fn logical_path(path: &Path) -> io::Result<PathBuf> {
    let absolute = std::path::absolute(path)?;
    Ok(dunce::simplified(&absolute).to_path_buf())
}

#[cfg(windows)]
fn send_windows_event(
    tx: &crossbeam_channel::Sender<VfsEvent>,
    symlink_index: &Mutex<WindowsSymlinkIndex>,
    path: PathBuf,
    kind: WindowsEventKind,
) -> Result<(), crossbeam_channel::SendError<VfsEvent>> {
    let normalized = logical_path(&path).ok();
    let currently_symlink = !matches!(kind, WindowsEventKind::Remove)
        && std::fs::symlink_metadata(&path)
            .map(|metadata| metadata.file_type().is_symlink())
            .unwrap_or(false);

    // Snapshot paths are keyed by physical targets, which handles content
    // events directly. Target removal is different: once Rojo removes the
    // corresponding instance, a later target creation has no tree key to find.
    // Rescanning the logical parent preserves that route for both halves of an
    // atomic replacement and for alias deletion/repointing.
    let rescans = {
        let index = symlink_index.lock().unwrap();
        if normalized
            .as_ref()
            .is_some_and(|path| index.logical_paths.contains(path))
            || currently_symlink
        {
            vec![path.clone()]
        } else if !matches!(kind, WindowsEventKind::Write) {
            normalized
                .as_ref()
                .and_then(|path| index.physical_to_logical.get(path))
                .map(|paths| paths.iter().cloned().collect())
                .unwrap_or_default()
        } else {
            Vec::new()
        }
    };

    if rescans.is_empty() {
        let event = match kind {
            WindowsEventKind::Create => VfsEvent::Create(path),
            WindowsEventKind::Write => VfsEvent::Write(path),
            WindowsEventKind::Remove => VfsEvent::Remove(path),
        };
        tx.send(event)
    } else {
        for path in rescans {
            tx.send(VfsEvent::Rescan(path))?;
        }
        Ok(())
    }
}

#[cfg(windows)]
fn minimal_watch_targets(paths: impl IntoIterator<Item = PathBuf>) -> HashSet<PathBuf> {
    let mut paths: Vec<_> = paths.into_iter().collect();
    paths.sort_by_key(|path| path.components().count());

    let mut minimal = HashSet::new();
    for path in paths {
        if !minimal.iter().any(|ancestor| path.starts_with(ancestor)) {
            minimal.insert(path);
        }
    }

    minimal
}

impl VfsBackend for StdBackend {
    fn read(&mut self, path: &Path) -> io::Result<Vec<u8>> {
        fs_err::read(path)
    }

    fn write(&mut self, path: &Path, data: &[u8]) -> io::Result<()> {
        fs_err::write(path, data)
    }

    fn exists(&mut self, path: &Path) -> io::Result<bool> {
        std::fs::exists(path)
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

    fn create_dir(&mut self, path: &Path) -> io::Result<()> {
        fs_err::create_dir(path)
    }

    fn create_dir_all(&mut self, path: &Path) -> io::Result<()> {
        fs_err::create_dir_all(path)
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

    fn canonicalize(&mut self, path: &Path) -> io::Result<PathBuf> {
        dunce::canonicalize(path)
    }

    fn event_receiver(&self) -> crossbeam_channel::Receiver<VfsEvent> {
        self.watcher_receiver.clone()
    }

    fn watch(&mut self, path: &Path) -> io::Result<()> {
        #[cfg(windows)]
        {
            self.watch_windows(path)
        }

        #[cfg(not(windows))]
        if self.watches.contains(path)
            || path
                .ancestors()
                .any(|ancestor| self.watches.contains(ancestor))
        {
            Ok(())
        } else {
            self.watches.insert(path.to_path_buf());
            self.watcher
                .watch(path, RecursiveMode::Recursive)
                .map_err(io::Error::other)
        }
    }

    fn unwatch(&mut self, path: &Path) -> io::Result<()> {
        #[cfg(windows)]
        {
            self.unwatch_windows(path)
        }

        #[cfg(not(windows))]
        {
            self.watches.remove(path);
            self.watcher.unwatch(path).map_err(io::Error::other)
        }
    }

    fn commit_event(&mut self, event: &VfsEvent) -> io::Result<()> {
        #[cfg(windows)]
        if let VfsEvent::Rescan(path) = event {
            return self.refresh_symlink_windows(path);
        }

        if let VfsEvent::Remove(path) = event {
            let _ = self.unwatch(path);
        }

        Ok(())
    }
}

#[cfg(all(test, windows))]
mod tests {
    use std::{
        collections::HashSet,
        os::windows::fs::{symlink_dir, symlink_file},
        path::Path,
        time::{Duration, Instant},
    };

    use crate::{StdBackend, Vfs, VfsEvent};

    fn event_path(event: &VfsEvent) -> &Path {
        match event {
            VfsEvent::Create(path)
            | VfsEvent::Write(path)
            | VfsEvent::Remove(path)
            | VfsEvent::Rescan(path) => path,
        }
    }

    fn same_path(left: &Path, right: &Path) -> bool {
        super::logical_path(left).ok() == super::logical_path(right).ok()
    }

    /// std has no API for creating junctions, and unlike symlinks they never
    /// need Developer Mode or elevation, so `mklink /J` works on every machine.
    fn create_junction(target: &Path, junction: &Path) {
        let status = std::process::Command::new("cmd")
            .arg("/C")
            .arg("mklink")
            .arg("/J")
            .arg(junction)
            .arg(target)
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(status.success(), "mklink /J failed: {}", status);
    }

    fn wait_for_event(
        vfs: &Vfs,
        events: &crossbeam_channel::Receiver<VfsEvent>,
        mut predicate: impl FnMut(&VfsEvent) -> bool,
    ) -> VfsEvent {
        let deadline = Instant::now() + Duration::from_secs(5);
        while let Ok(event) = events.recv_deadline(deadline) {
            vfs.commit_event(&event).unwrap();
            if predicate(&event) {
                return event;
            }
        }

        panic!("timed out waiting for filesystem event");
    }

    fn drain_events(vfs: &Vfs, events: &crossbeam_channel::Receiver<VfsEvent>) {
        while let Ok(event) = events.recv_timeout(Duration::from_millis(150)) {
            vfs.commit_event(&event).unwrap();
        }
    }

    fn assert_no_event_for_path(
        vfs: &Vfs,
        events: &crossbeam_channel::Receiver<VfsEvent>,
        path: &Path,
    ) {
        let deadline = Instant::now() + Duration::from_millis(750);
        while let Ok(event) = events.recv_deadline(deadline) {
            vfs.commit_event(&event).unwrap();
            assert!(
                !same_path(event_path(&event), path),
                "unexpected watcher event for {}: {:?}",
                path.display(),
                event
            );
        }
    }

    #[test]
    fn directory_symlink_reports_physical_target_write_after_parent_watch() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let target = temp.path().join("target");
        let alias = source.join("alias");
        let target_file = target.join("child.txt");

        std::fs::create_dir(&source).unwrap();
        std::fs::create_dir(&target).unwrap();
        std::fs::write(&target_file, "before").unwrap();
        symlink_dir(&target, &alias)
            .expect("this Windows regression test requires permission to create symlinks");

        let vfs = Vfs::new(StdBackend::new().unwrap());
        let events = vfs.event_receiver();
        vfs.read_dir(&source)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        vfs.read_dir(&alias)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        std::fs::write(&target_file, "after").unwrap();

        wait_for_event(&vfs, &events, |event| {
            same_path(event_path(event), &target_file)
        });
    }

    #[test]
    fn directory_symlink_reports_alias_child_lifecycle() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let target = temp.path().join("target");
        let alias = source.join("alias");
        let created = target.join("created.lua");
        let renamed = target.join("renamed.lua");

        std::fs::create_dir(&source).unwrap();
        std::fs::create_dir(&target).unwrap();
        symlink_dir(&target, &alias)
            .expect("this Windows regression test requires permission to create symlinks");

        let vfs = Vfs::new(StdBackend::new().unwrap());
        let events = vfs.event_receiver();
        vfs.read_dir(&source).unwrap();
        vfs.read_dir(&alias).unwrap();

        std::fs::write(alias.join("created.lua"), "created").unwrap();
        wait_for_event(&vfs, &events, |event| {
            matches!(event, VfsEvent::Create(_) | VfsEvent::Write(_))
                && same_path(event_path(event), &created)
        });
        drain_events(&vfs, &events);

        std::fs::rename(alias.join("created.lua"), alias.join("renamed.lua")).unwrap();
        let mut saw_remove = false;
        let mut saw_create = false;
        let deadline = Instant::now() + Duration::from_secs(5);
        while !(saw_remove && saw_create) {
            let event = events
                .recv_deadline(deadline)
                .expect("timed out waiting for rename events");
            vfs.commit_event(&event).unwrap();
            saw_remove |=
                matches!(&event, VfsEvent::Remove(_)) && same_path(event_path(&event), &created);
            saw_create |=
                matches!(&event, VfsEvent::Create(_)) && same_path(event_path(&event), &renamed);
        }

        std::fs::remove_file(alias.join("renamed.lua")).unwrap();
        wait_for_event(&vfs, &events, |event| {
            matches!(event, VfsEvent::Remove(_)) && same_path(event_path(event), &renamed)
        });
    }

    #[test]
    fn file_symlink_reports_physical_and_alias_writes() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let target = temp.path().join("target.lua");
        let alias = source.join("alias.lua");

        std::fs::create_dir(&source).unwrap();
        std::fs::write(&target, "before").unwrap();
        symlink_file(&target, &alias)
            .expect("this Windows regression test requires permission to create symlinks");

        let vfs = Vfs::new(StdBackend::new().unwrap());
        let events = vfs.event_receiver();
        vfs.read_dir(&source).unwrap();
        vfs.read_to_string(&alias).unwrap();

        std::fs::write(&target, "physical").unwrap();
        wait_for_event(&vfs, &events, |event| same_path(event_path(event), &target));
        drain_events(&vfs, &events);

        std::fs::write(&alias, "alias").unwrap();
        wait_for_event(&vfs, &events, |event| same_path(event_path(event), &target));
    }

    #[test]
    fn directory_symlink_recreation_and_repointing_refresh_watches() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let target_a = temp.path().join("target-a");
        let target_b = temp.path().join("target-b");
        let alias = source.join("alias");

        std::fs::create_dir(&source).unwrap();
        std::fs::create_dir(&target_a).unwrap();
        std::fs::create_dir(&target_b).unwrap();
        symlink_dir(&target_a, &alias)
            .expect("this Windows regression test requires permission to create symlinks");

        let vfs = Vfs::new(StdBackend::new().unwrap());
        let events = vfs.event_receiver();
        vfs.read_dir(&source).unwrap();
        vfs.read_dir(&alias).unwrap();

        std::fs::remove_dir(&alias).unwrap();
        wait_for_event(
            &vfs,
            &events,
            |event| matches!(event, VfsEvent::Rescan(path) if same_path(path, &alias)),
        );

        symlink_dir(&target_a, &alias).unwrap();
        wait_for_event(
            &vfs,
            &events,
            |event| matches!(event, VfsEvent::Rescan(path) if same_path(path, &alias)),
        );
        let recreated_file = target_a.join("recreated.lua");
        std::fs::write(&recreated_file, "recreated").unwrap();
        wait_for_event(&vfs, &events, |event| {
            same_path(event_path(event), &recreated_file)
        });
        drain_events(&vfs, &events);

        std::fs::remove_dir(&alias).unwrap();
        wait_for_event(
            &vfs,
            &events,
            |event| matches!(event, VfsEvent::Rescan(path) if same_path(path, &alias)),
        );
        symlink_dir(&target_b, &alias).unwrap();
        wait_for_event(
            &vfs,
            &events,
            |event| matches!(event, VfsEvent::Rescan(path) if same_path(path, &alias)),
        );

        let new_target_file = target_b.join("new-target.lua");
        std::fs::write(&new_target_file, "new target").unwrap();
        wait_for_event(&vfs, &events, |event| {
            same_path(event_path(event), &new_target_file)
        });
        drain_events(&vfs, &events);

        let stale_target_file = target_a.join("stale-target.lua");
        std::fs::write(&stale_target_file, "stale target").unwrap();
        assert_no_event_for_path(&vfs, &events, &stale_target_file);
    }

    #[test]
    fn shared_target_watch_survives_until_last_alias_is_removed() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let target = temp.path().join("target");
        let alias_a = source.join("alias-a");
        let alias_b = source.join("alias-b");

        std::fs::create_dir(&source).unwrap();
        std::fs::create_dir(&target).unwrap();
        symlink_dir(&target, &alias_a)
            .expect("this Windows regression test requires permission to create symlinks");
        symlink_dir(&target, &alias_b).unwrap();

        let vfs = Vfs::new(StdBackend::new().unwrap());
        let events = vfs.event_receiver();
        vfs.read_dir(&source).unwrap();
        vfs.read_dir(&alias_a).unwrap();
        vfs.read_dir(&alias_b).unwrap();

        std::fs::remove_dir(&alias_a).unwrap();
        wait_for_event(
            &vfs,
            &events,
            |event| matches!(event, VfsEvent::Rescan(path) if same_path(path, &alias_a)),
        );

        let still_watched = target.join("still-watched.lua");
        std::fs::write(&still_watched, "still watched").unwrap();
        wait_for_event(&vfs, &events, |event| {
            same_path(event_path(event), &still_watched)
        });
        drain_events(&vfs, &events);

        std::fs::remove_dir(&alias_b).unwrap();
        wait_for_event(
            &vfs,
            &events,
            |event| matches!(event, VfsEvent::Rescan(path) if same_path(path, &alias_b)),
        );
        drain_events(&vfs, &events);

        let no_longer_watched = target.join("no-longer-watched.lua");
        std::fs::write(&no_longer_watched, "not watched").unwrap();
        assert_no_event_for_path(&vfs, &events, &no_longer_watched);
    }

    #[test]
    fn directory_junction_reports_physical_target_lifecycle() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let target = temp.path().join("target");
        let junction = source.join("junction");
        let target_file = target.join("child.lua");
        let created = target.join("created.lua");

        std::fs::create_dir(&source).unwrap();
        std::fs::create_dir(&target).unwrap();
        std::fs::write(&target_file, "before").unwrap();
        create_junction(&target, &junction);

        let vfs = Vfs::new(StdBackend::new().unwrap());
        let events = vfs.event_receiver();
        vfs.read_dir(&source).unwrap();
        vfs.read_dir(&junction).unwrap();

        std::fs::write(&target_file, "after").unwrap();
        wait_for_event(&vfs, &events, |event| {
            same_path(event_path(event), &target_file)
        });
        drain_events(&vfs, &events);

        std::fs::write(&created, "created").unwrap();
        wait_for_event(&vfs, &events, |event| {
            matches!(event, VfsEvent::Create(_) | VfsEvent::Write(_))
                && same_path(event_path(event), &created)
        });
    }

    #[test]
    fn ordinary_directory_events_still_work() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let file = source.join("ordinary.lua");
        std::fs::create_dir(&source).unwrap();

        let vfs = Vfs::new(StdBackend::new().unwrap());
        let events = vfs.event_receiver();
        vfs.read_dir(&source).unwrap();

        std::fs::write(&file, "ordinary").unwrap();
        wait_for_event(&vfs, &events, |event| {
            matches!(event, VfsEvent::Create(_) | VfsEvent::Write(_))
                && same_path(event_path(event), &file)
        });
    }

    #[test]
    fn minimal_targets_deduplicate_aliases_and_descendants() {
        let root = Path::new(r"C:\source");
        let target = Path::new(r"C:\shared");
        let targets = super::minimal_watch_targets([
            root.to_path_buf(),
            root.join("child"),
            target.to_path_buf(),
            target.to_path_buf(),
            target.join("nested"),
        ]);

        assert_eq!(
            targets,
            HashSet::from([root.to_path_buf(), target.to_path_buf()])
        );
    }
}
