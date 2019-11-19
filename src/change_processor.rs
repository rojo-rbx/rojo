//! Defines the process by which changes are pulled from the Vfs, filtered, and
//! used to mutate Rojo's tree during a live session.
//!
//! This object is owned by a ServeSession.

use std::sync::{Arc, Mutex};

use crossbeam_channel::{select, Receiver, Sender};
use jod_thread::JoinHandle;
use rbx_dom_weak::RbxId;

use crate::{
    message_queue::MessageQueue,
    snapshot::{
        apply_patch_set, compute_patch_set, AppliedPatchSet, InstigatingSource, PatchSet, RojoTree,
    },
    snapshot_middleware::{snapshot_from_vfs, snapshot_project_node, InstanceSnapshotContext},
    vfs::{FsResultExt, Vfs, VfsEvent, VfsFetcher},
};

pub struct ChangeProcessor {
    shutdown_sender: Sender<()>,
    _thread_handle: JoinHandle<()>,
}

impl ChangeProcessor {
    pub fn start<F: VfsFetcher + Send + Sync + 'static>(
        tree: Arc<Mutex<RojoTree>>,
        message_queue: Arc<MessageQueue<AppliedPatchSet>>,
        vfs: Arc<Vfs<F>>,
    ) -> Self {
        let (shutdown_sender, shutdown_receiver) = crossbeam_channel::bounded(1);

        let thread_handle = jod_thread::Builder::new()
            .name("ChangeProcessor thread".to_owned())
            .spawn(move || {
                log::trace!("ChangeProcessor thread started");
                Self::main_task(shutdown_receiver, tree, message_queue, vfs);
                log::trace!("ChangeProcessor thread stopped");
            })
            .expect("Could not start ChangeProcessor thread");

        Self {
            shutdown_sender,
            _thread_handle: thread_handle,
        }
    }

    fn main_task<F: VfsFetcher>(
        shutdown_receiver: Receiver<()>,
        tree: Arc<Mutex<RojoTree>>,
        message_queue: Arc<MessageQueue<AppliedPatchSet>>,
        vfs: Arc<Vfs<F>>,
    ) {
        let vfs_receiver = vfs.change_receiver();

        #[allow(
            // Crossbeam's select macro generates code that Clippy doesn't like,
            // and Clippy blames us for it.
            clippy::drop_copy,

            // Crossbeam uses 0 as *const _ and Clippy doesn't like that either,
            // but this isn't our fault.
            clippy::zero_ptr,
        )]
        loop {
            select! {
                recv(vfs_receiver) -> event => {
                    let event = event.unwrap();

                    log::trace!("Vfs event: {:?}", event);

                    let applied_patches = {
                        vfs.commit_change(&event).expect("Error applying VFS change");

                        let mut tree = tree.lock().unwrap();
                        let mut applied_patches = Vec::new();

                        match event {
                            VfsEvent::Created(path) | VfsEvent::Modified(path) | VfsEvent::Removed(path) => {
                                let mut current_path = path.as_path();
                                let affected_ids = loop {
                                    let ids = tree.get_ids_at_path(&current_path);

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

                                update_affected_instances(&mut tree, &vfs, &mut applied_patches, &affected_ids)
                            }
                        }

                        applied_patches
                    };

                    {
                        message_queue.push_messages(&applied_patches);
                    }
                },
                recv(shutdown_receiver) -> _ => {
                    log::trace!("ChangeProcessor shutdown signal received...");
                    break;
                },
            }
        }
    }
}

impl Drop for ChangeProcessor {
    fn drop(&mut self) {
        let _ = self.shutdown_sender.send(());
    }
}

fn update_affected_instances<F: VfsFetcher>(
    tree: &mut RojoTree,
    vfs: &Vfs<F>,
    applied_patches: &mut Vec<AppliedPatchSet>,
    affected_ids: &[RbxId],
) {
    for &id in affected_ids {
        let metadata = tree
            .get_metadata(id)
            .expect("metadata missing for instance present in tree");

        let instigating_source = match &metadata.instigating_source {
            Some(path) => path,
            None => {
                log::warn!("Instance {} did not have an instigating source, but was considered for an update.", id);
                log::warn!("This is a Rojo bug. Please file an issue!");
                continue;
            }
        };

        // TODO: Use persisted snapshot context struct instead of recreating it
        // every time.
        let mut snapshot_context = InstanceSnapshotContext::default();

        // How we process a file change event depends on what created this
        // file/folder in the first place.
        let applied_patch_set = match instigating_source {
            InstigatingSource::Path(path) => {
                let maybe_entry = vfs
                    .get(path)
                    .with_not_found()
                    .expect("unexpected VFS error");

                match maybe_entry {
                    Some(entry) => {
                        // Our instance was previously created from a path and
                        // that path still exists. We can generate a snapshot
                        // starting at that path and use it as the source for
                        // our patch.

                        let snapshot = snapshot_from_vfs(&mut snapshot_context, &vfs, &entry)
                            .expect("snapshot failed")
                            .expect("snapshot did not return an instance");

                        let patch_set = compute_patch_set(&snapshot, &tree, id);
                        apply_patch_set(tree, patch_set)
                    }
                    None => {
                        // Our instance was previously created from a path, but
                        // that path no longer exists.
                        //
                        // We associate deleting the instigating file for an
                        // instance with deleting that instance.

                        let mut patch_set = PatchSet::new();
                        patch_set.removed_instances.push(id);

                        apply_patch_set(tree, patch_set)
                    }
                }
            }
            InstigatingSource::ProjectNode(instance_name, project_node) => {
                // This instance is the direct subject of a project node. Since
                // there might be information associated with our instance from
                // the project file, we snapshot the entire project node again.

                let snapshot =
                    snapshot_project_node(&mut snapshot_context, instance_name, project_node, &vfs)
                        .expect("snapshot failed")
                        .expect("snapshot did not return an instance");

                let patch_set = compute_patch_set(&snapshot, &tree, id);
                apply_patch_set(tree, patch_set)
            }
        };

        applied_patches.push(applied_patch_set);
    }
}
