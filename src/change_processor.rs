//! Defines the process by which changes are pulled from the Vfs, filtered, and
//! used to mutate Rojo's tree during a live session.
//!
//! This object is owned by a ServeSession.

use std::sync::{Arc, Mutex};

use crossbeam_channel::{select, Receiver, Sender};
use jod_thread::JoinHandle;

use crate::{
    message_queue::MessageQueue,
    snapshot::{apply_patch_set, compute_patch_set, AppliedPatchSet, InstigatingSource, RojoTree},
    snapshot_middleware::{snapshot_from_vfs, InstanceSnapshotContext},
    vfs::{Vfs, VfsEvent, VfsFetcher},
};

pub struct ChangeProcessor {
    shutdown_sender: Sender<()>,
    _thread_handle: JoinHandle<()>,
}

impl ChangeProcessor {
    pub fn start<F: VfsFetcher + Send + 'static>(
        tree: Arc<Mutex<RojoTree>>,
        message_queue: Arc<MessageQueue<AppliedPatchSet>>,
        vfs: Arc<Mutex<Vfs<F>>>,
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
        vfs: Arc<Mutex<Vfs<F>>>,
    ) {
        let vfs_receiver = {
            let vfs = vfs.lock().unwrap();
            vfs.change_receiver()
        };

        // Crossbeam's select macro generates code that Clippy doesn't like, and
        // Clippy blames us for it.
        #[allow(clippy::drop_copy)]
        loop {
            select! {
                recv(vfs_receiver) -> event => {
                    let event = event.unwrap();

                    log::trace!("Vfs event: {:?}", event);

                    let applied_patches = {
                        let mut vfs = vfs.lock().unwrap();
                        vfs.commit_change(&event).expect("Error applying VFS change");

                        let mut tree = tree.lock().unwrap();
                        let mut applied_patches = Vec::new();

                        match event {
                            VfsEvent::Created(path) | VfsEvent::Modified(path) | VfsEvent::Removed(path) => {
                                let affected_ids = tree.get_ids_at_path(&path).to_vec();

                                if affected_ids.len() == 0 {
                                    log::info!("No instances were affected by this change.");
                                    continue;
                                }

                                for id in affected_ids {
                                    let metadata = tree.get_metadata(id)
                                        .expect("metadata missing for instance present in tree");

                                    let instigating_source = match &metadata.instigating_source {
                                        Some(path) => path,
                                        None => {
                                            log::warn!("Instance {} did not have an instigating source, but was considered for an update.", id);
                                            log::warn!("This is a Rojo bug. Please file an issue!");
                                            continue;
                                        }
                                    };

                                    let snapshot = match instigating_source {
                                        InstigatingSource::Path(path) => {
                                            let entry = vfs
                                                .get(path)
                                                .expect("could not get instigating path from filesystem");

                                            // TODO: Use persisted snapshot
                                            // context struct instead of
                                            // recreating it every time.
                                            let snapshot = snapshot_from_vfs(&mut InstanceSnapshotContext::default(), &mut vfs, &entry)
                                                .expect("snapshot failed")
                                                .expect("snapshot did not return an instance");

                                            snapshot
                                        }
                                        InstigatingSource::ProjectNode(_, _) => {
                                            log::warn!("Instance {} had an instigating source that was a project node, which is not yet supported.", id);
                                            continue;
                                        }
                                    };

                                    log::trace!("Computed snapshot: {:#?}", snapshot);

                                    let patch_set = compute_patch_set(&snapshot, &tree, id);
                                    let applied_patch_set = apply_patch_set(&mut tree, patch_set);

                                    log::trace!("Applied patch: {:#?}", applied_patch_set);

                                    applied_patches.push(applied_patch_set);
                                }
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
