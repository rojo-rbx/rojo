use crossbeam_channel::{select, Receiver, RecvError, Sender};
use jod_thread::JoinHandle;
use memofs::{IoResultExt, Vfs, VfsEvent};
use rbx_dom_weak::types::{Ref, Variant};
use std::path::{Path, PathBuf};
use std::{
    fs,
    sync::{Arc, Mutex},
};

use crate::{
    message_queue::MessageQueue,
    snapshot::{
        apply_patch_set, compute_patch_set, AppliedPatchSet, InstigatingSource, PatchSet, RojoTree,
    },
    snapshot_middleware::{snapshot_from_vfs, snapshot_project_node},
};

/// Processes file change events, updates the DOM, and sends those updates
/// through a channel for other stuff to consume.
///
/// Owns the connection between Rojo's VFS and its DOM by holding onto another
/// thread that processes messages.
///
/// Consumers of ChangeProcessor, like ServeSession, are intended to communicate
/// with this object via channels.
///
/// ChangeProcessor expects to be the only writer to the RojoTree and Vfs
/// objects passed to it.
pub struct ChangeProcessor {
    /// Controls the runtime of the processor thread. When signaled, the job
    /// thread will finish its current work and terminate.
    ///
    /// This channel should be signaled before dropping ChangeProcessor or we'll
    /// hang forever waiting for the message processing loop to terminate.
    shutdown_sender: Sender<()>,

    /// A handle to the message processing thread. When dropped, we'll block
    /// until it's done.
    ///
    /// Allowed to be unused because dropping this value has side effects.
    #[allow(unused)]
    job_thread: JoinHandle<Result<(), RecvError>>,
}

impl ChangeProcessor {
    /// Spin up the ChangeProcessor, connecting it to the given tree, VFS, and
    /// outbound message queue.
    pub fn start(
        tree: Arc<Mutex<RojoTree>>,
        vfs: Arc<Vfs>,
        message_queue: Arc<MessageQueue<AppliedPatchSet>>,
        tree_mutation_receiver: Receiver<PatchSet>,
    ) -> Self {
        let (shutdown_sender, shutdown_receiver) = crossbeam_channel::bounded(1);
        let vfs_receiver = vfs.event_receiver();
        let task = JobThreadContext {
            tree,
            vfs,
            message_queue,
        };

        let job_thread = jod_thread::Builder::new()
            .name("ChangeProcessor thread".to_owned())
            .spawn(move || {
                log::trace!("ChangeProcessor thread started");

                loop {
                    select! {
                        recv(vfs_receiver) -> event => {
                            task.handle_vfs_event(event?);
                        },
                        recv(tree_mutation_receiver) -> patch_set => {
                            task.handle_tree_event(patch_set?);
                        },
                        recv(shutdown_receiver) -> _ => {
                            log::trace!("ChangeProcessor shutdown signal received...");
                            return Ok(());
                        },
                    }
                }
            })
            .expect("Could not start ChangeProcessor thread");

        Self {
            shutdown_sender,
            job_thread,
        }
    }
}

impl Drop for ChangeProcessor {
    fn drop(&mut self) {
        // Signal the job thread to start spinning down. Without this we'll hang
        // forever waiting for the thread to finish its infinite loop.
        let _ = self.shutdown_sender.send(());

        // After this function ends, the job thread will be joined. It might
        // block for a small period of time while it processes its last work.
    }
}

/// Contains all of the state needed to synchronize the DOM and VFS.
struct JobThreadContext {
    /// A handle to the DOM we're managing.
    tree: Arc<Mutex<RojoTree>>,

    /// A handle to the VFS we're managing.
    vfs: Arc<Vfs>,

    /// Whenever changes are applied to the DOM, we should push those changes
    /// into this message queue to inform any connected clients.
    message_queue: Arc<MessageQueue<AppliedPatchSet>>,
}

impl JobThreadContext {
    /// Computes and applies patches to the DOM for a given file path.
    ///
    /// This function finds the nearest ancestor to the given path that has associated instances
    /// in the tree.
    /// It then computes and applies changes for each affected instance ID and
    /// returns a vector of applied patch sets.
    fn apply_patches(&self, path: PathBuf) -> Vec<AppliedPatchSet> {
        let mut tree = self.tree.lock().unwrap();
        let mut applied_patches = Vec::new();

        // Find the nearest ancestor to this path that has
        // associated instances in the tree. This helps make sure
        // that we handle additions correctly, especially if we
        // receive events for descendants of a large tree being
        // created all at once.
        let mut current_path = path.as_path();
        let affected_ids = loop {
            let ids = tree.get_ids_at_path(current_path);

            log::trace!("Path {} affects IDs {:?}", current_path.display(), ids);

            if !ids.is_empty() {
                break ids.to_vec();
            }

            log::trace!("Trying parent path...");
            match current_path.parent() {
                Some(parent) => current_path = parent,
                None => break Vec::new(),
            }
        };

        for id in affected_ids {
            if let Some(patch) = compute_and_apply_changes(&mut tree, &self.vfs, id) {
                if !patch.is_empty() {
                    applied_patches.push(patch);
                }
            }
        }

        applied_patches
    }

    fn handle_vfs_event(&self, event: VfsEvent) {
        log::trace!("Vfs event: {:?}", event);

        // Update the VFS immediately with the event.
        self.vfs
            .commit_event(&event)
            .expect("Error applying VFS change");

        // For a given VFS event, we might have many changes to different parts
        // of the tree. Calculate and apply all of these changes.
        let applied_patches = match event {
            VfsEvent::Create(path) | VfsEvent::Write(path) | VfsEvent::Remove(path) => {
                self.apply_patches(canonicalize_best_effort(&self.vfs, &path))
            }
            _ => {
                log::warn!("Unhandled VFS event: {:?}", event);
                Vec::new()
            }
        };

        // Notify anyone listening to the message queue about the changes we
        // just made.
        self.message_queue.push_messages(&applied_patches);
    }

    fn handle_tree_event(&self, patch_set: PatchSet) {
        log::trace!("Applying PatchSet from client: {:#?}", patch_set);

        let applied_patch = {
            let mut tree = self.tree.lock().unwrap();

            for &id in &patch_set.removed_instances {
                if let Some(instance) = tree.get_instance(id) {
                    if let Some(instigating_source) = &instance.metadata().instigating_source {
                        match instigating_source {
                            InstigatingSource::Path(path) => {
                                if let Err(err) = fs::remove_file(path) {
                                    log::error!(
                                        "Failed to remove file {}: {}",
                                        path.display(),
                                        err
                                    );
                                }
                            }
                            InstigatingSource::ProjectNode { .. } => {
                                log::warn!(
                                    "Cannot remove instance {:?}, it's from a project file",
                                    id
                                );
                            }
                        }
                    } else {
                        // TODO
                        log::warn!(
                            "Cannot remove instance {:?}, it is not an instigating source.",
                            id
                        );
                    }
                } else {
                    log::warn!("Cannot remove instance {:?}, it does not exist.", id);
                }
            }

            for update in &patch_set.updated_instances {
                let id = update.id;

                if let Some(instance) = tree.get_instance(id) {
                    if update.changed_name.is_some() {
                        log::warn!("Cannot rename instances yet.");
                    }

                    if update.changed_class_name.is_some() {
                        log::warn!("Cannot change ClassName yet.");
                    }

                    if update.changed_metadata.is_some() {
                        log::warn!("Cannot change metadata yet.");
                    }

                    for (key, changed_value) in &update.changed_properties {
                        if key == "Source" {
                            if let Some(instigating_source) =
                                &instance.metadata().instigating_source
                            {
                                match instigating_source {
                                    InstigatingSource::Path(path) => {
                                        if let Some(Variant::String(value)) = changed_value {
                                            if let Err(err) = fs::write(path, value) {
                                                log::error!(
                                                    "Failed to write file {}: {}",
                                                    path.display(),
                                                    err
                                                );
                                            }
                                        } else {
                                            log::warn!("Cannot change Source to non-string value.");
                                        }
                                    }
                                    InstigatingSource::ProjectNode { .. } => {
                                        log::warn!(
                                            "Cannot remove instance {:?}, it's from a project file",
                                            id
                                        );
                                    }
                                }
                            } else {
                                log::warn!(
                                    "Cannot update instance {:?}, it is not an instigating source.",
                                    id
                                );
                            }
                        } else {
                            log::warn!("Cannot change properties besides BaseScript.Source.");
                        }
                    }
                } else {
                    log::warn!("Cannot update instance {:?}, it does not exist.", id);
                }
            }

            apply_patch_set(&mut tree, patch_set)
        };

        if !applied_patch.is_empty() {
            self.message_queue.push_messages(&[applied_patch]);
        }
    }
}

/// Canonicalizes the deepest ancestor of `path` that can be canonicalized and
/// appends the rest of `path` to it.
///
/// Events are processed some time after they happen on disk, so their paths
/// may no longer exist: a file can be created and deleted again, and a
/// recursive delete removes a directory right after its children. Paths in
/// the tree are canonical, so we still need to canonicalize what we can for
/// them to match. Any error is treated as the path being gone, since on
/// Windows a file that is being deleted can fail with other errors too.
fn canonicalize_best_effort(vfs: &Vfs, path: &Path) -> PathBuf {
    if let Ok(canonical) = vfs.canonicalize(path) {
        return canonical;
    }

    match (path.parent(), path.file_name()) {
        (Some(parent), Some(file_name)) => canonicalize_best_effort(vfs, parent).join(file_name),
        _ => path.to_path_buf(),
    }
}

fn compute_and_apply_changes(tree: &mut RojoTree, vfs: &Vfs, id: Ref) -> Option<AppliedPatchSet> {
    let metadata = tree
        .get_metadata(id)
        .expect("metadata missing for instance present in tree");

    let instigating_source = match &metadata.instigating_source {
        Some(path) => path,
        None => {
            log::error!(
                "Instance {:?} did not have an instigating source, but was considered for an update.",
                id
            );
            log::error!("This is a bug. Please file an issue!");
            return None;
        }
    };

    // How we process a file change event depends on what created this
    // file/folder in the first place.
    let applied_patch_set = match instigating_source {
        InstigatingSource::Path(path) => match vfs.metadata(path).with_not_found() {
            Ok(Some(_)) => {
                // Our instance was previously created from a path and that
                // path still exists. We can generate a snapshot starting at
                // that path and use it as the source for our patch.

                let snapshot = match snapshot_from_vfs(&metadata.context, vfs, path) {
                    Ok(snapshot) => snapshot,
                    Err(err) => {
                        log::error!("Snapshot error: {:?}", err);
                        return None;
                    }
                };

                let patch_set = compute_patch_set(snapshot, tree, id);
                apply_patch_set(tree, patch_set)
            }
            Ok(None) => {
                // Our instance was previously created from a path, but that
                // path no longer exists.
                //
                // We associate deleting the instigating file for an
                // instance with deleting that instance.

                let mut patch_set = PatchSet::new();
                patch_set.removed_instances.push(id);

                apply_patch_set(tree, patch_set)
            }
            Err(err) => {
                log::error!("Error processing filesystem change: {:?}", err);
                return None;
            }
        },

        InstigatingSource::ProjectNode {
            path,
            name,
            node,
            parent_class,
        } => {
            // This instance is the direct subject of a project node. Since
            // there might be information associated with our instance from
            // the project file, we snapshot the entire project node again.

            let snapshot_result = snapshot_project_node(
                &metadata.context,
                path,
                name,
                node,
                vfs,
                parent_class.as_ref().map(|name| name.as_str()),
            );

            let snapshot = match snapshot_result {
                Ok(snapshot) => snapshot,
                Err(err) => {
                    log::error!("{:?}", err);
                    return None;
                }
            };

            let patch_set = compute_patch_set(snapshot, tree, id);
            apply_patch_set(tree, patch_set)
        }
    };

    Some(applied_patch_set)
}

#[cfg(test)]
mod test {
    use super::*;

    use memofs::StdBackend;
    use tempfile::TempDir;

    use crate::snapshot::{InstanceContext, InstanceSnapshot};

    /// Creates a project in a temporary directory and returns its root. The
    /// root is canonicalized the same way the VFS does, so that it matches the
    /// paths in the tree on every platform.
    fn make_project() -> (TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap().join("project");

        let comm = root.join("src").join("Packages").join("knit").join("Comm");
        fs::create_dir_all(&comm).unwrap();
        fs::write(
            root.join("default.project.json"),
            r#"{ "name": "test", "tree": { "$path": "src" } }"#,
        )
        .unwrap();
        fs::write(comm.parent().unwrap().join("init.luau"), "return {}").unwrap();
        fs::write(comm.join("init.luau"), "return {}").unwrap();

        (dir, root)
    }

    /// Builds a tree for the project at `root` like `ServeSession` does, so
    /// that VFS events can be fed to it directly in whatever order a test
    /// needs.
    fn context_for(root: &Path) -> JobThreadContext {
        let vfs = Vfs::new(StdBackend::new().unwrap());
        vfs.set_watch_enabled(false);

        let mut tree = RojoTree::new(InstanceSnapshot::new());
        let root_id = tree.get_root_id();
        let snapshot = snapshot_from_vfs(&InstanceContext::new(), &vfs, root).unwrap();
        let patch_set = compute_patch_set(snapshot, &tree, root_id);
        apply_patch_set(&mut tree, patch_set);

        JobThreadContext {
            tree: Arc::new(Mutex::new(tree)),
            vfs: Arc::new(vfs),
            message_queue: Arc::new(MessageQueue::new()),
        }
    }

    fn is_in_tree(context: &JobThreadContext, path: &Path) -> bool {
        !context
            .tree
            .lock()
            .unwrap()
            .get_ids_at_path(path)
            .is_empty()
    }

    /// Sends `Remove` events for `knit` and everything in it, children first,
    /// the way a recursive delete reports them.
    fn send_knit_removals(context: &JobThreadContext, knit: &Path) {
        let comm = knit.join("Comm");
        for path in [
            comm.join("init.luau"),
            comm,
            knit.join("init.luau"),
            knit.to_path_buf(),
        ] {
            context.handle_vfs_event(VfsEvent::Remove(path));
        }
    }

    /// https://github.com/rojo-rbx/rojo/issues/1338
    ///
    /// By the time we process the event for a removed file, a recursive
    /// delete may have removed its parent as well.
    #[test]
    fn remove_events_after_parent_is_gone() {
        let (_dir, root) = make_project();
        let context = context_for(&root);
        let packages = root.join("src").join("Packages");
        let knit = packages.join("knit");
        assert!(is_in_tree(&context, &knit));

        fs::remove_dir_all(&knit).unwrap();
        send_knit_removals(&context, &knit);

        assert!(!is_in_tree(&context, &knit));
        assert!(is_in_tree(&context, &packages));
    }

    /// https://github.com/rojo-rbx/rojo/issues/1349
    ///
    /// A file can be written to and then deleted before we process the write.
    #[test]
    fn write_event_after_path_is_gone() {
        let (_dir, root) = make_project();
        let context = context_for(&root);
        let comm = root.join("src").join("Packages").join("knit").join("Comm");
        assert!(is_in_tree(&context, &comm));

        fs::remove_dir_all(&comm).unwrap();
        context.handle_vfs_event(VfsEvent::Write(comm.join("init.luau")));

        assert!(!is_in_tree(&context, &comm));
    }

    /// Events can name a path differently from the tree, which is why event
    /// paths are canonicalized. That still has to happen when the path is
    /// gone, or the event won't match anything in the tree.
    #[cfg(unix)]
    #[test]
    fn remove_events_through_symlink_after_parent_is_gone() {
        let (dir, root) = make_project();
        let context = context_for(&root);
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&root, &link).unwrap();

        let knit = root.join("src").join("Packages").join("knit");
        let linked_knit = link.join("src").join("Packages").join("knit");

        fs::remove_dir_all(&linked_knit).unwrap();
        send_knit_removals(&context, &linked_knit);

        assert!(!is_in_tree(&context, &knit));
    }
}
