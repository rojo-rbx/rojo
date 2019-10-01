//! Defines the process by which changes are pulled from the Imfs, filtered, and
//! used to mutate Rojo's tree during a live session.
//!
//! This object is owned by a ServeSession.

use std::sync::{Arc, Mutex};

use crossbeam_channel::{select, Receiver, Sender};
use jod_thread::JoinHandle;

use crate::{
    imfs::{Imfs, ImfsEvent, ImfsFetcher},
    message_queue::MessageQueue,
    snapshot::{apply_patch_set, compute_patch_set, AppliedPatchSet, RojoTree},
    snapshot_middleware::snapshot_from_imfs,
};

pub struct ChangeProcessor {
    shutdown_sender: Sender<()>,
    _thread_handle: JoinHandle<()>,
}

impl ChangeProcessor {
    pub fn start<F: ImfsFetcher + Send + 'static>(
        tree: Arc<Mutex<RojoTree>>,
        message_queue: Arc<MessageQueue<AppliedPatchSet>>,
        imfs: Arc<Mutex<Imfs<F>>>,
    ) -> Self {
        let (shutdown_sender, shutdown_receiver) = crossbeam_channel::bounded(1);

        let thread_handle = jod_thread::Builder::new()
            .name("ChangeProcessor thread".to_owned())
            .spawn(move || {
                log::trace!("ChangeProcessor thread started");
                Self::main_task(shutdown_receiver, tree, message_queue, imfs);
                log::trace!("ChangeProcessor thread stopped");
            })
            .expect("Could not start ChangeProcessor thread");

        Self {
            shutdown_sender,
            _thread_handle: thread_handle,
        }
    }

    fn main_task<F: ImfsFetcher>(
        shutdown_receiver: Receiver<()>,
        tree: Arc<Mutex<RojoTree>>,
        message_queue: Arc<MessageQueue<AppliedPatchSet>>,
        imfs: Arc<Mutex<Imfs<F>>>,
    ) {
        let imfs_receiver = {
            let imfs = imfs.lock().unwrap();
            imfs.change_receiver()
        };

        // Crossbeam's select macro generates code that Clippy doesn't like, and
        // Clippy blames us for it.
        #[allow(clippy::drop_copy)]
        loop {
            select! {
                recv(imfs_receiver) -> event => {
                    let event = event.unwrap();

                    log::trace!("Imfs event: {:?}", event);

                    let applied_patches = {
                        let mut imfs = imfs.lock().unwrap();
                        imfs.commit_change(&event).expect("Error applying IMFS change");

                        let mut tree = tree.lock().unwrap();
                        let mut applied_patches = Vec::new();

                        match event {
                            ImfsEvent::Created(path) | ImfsEvent::Modified(path) | ImfsEvent::Removed(path) => {
                                let affected_ids = tree.get_ids_at_path(&path).to_vec();

                                if affected_ids.len() == 0 {
                                    log::info!("No instances were affected by this change.");
                                    continue;
                                }

                                for id in affected_ids {
                                    let metadata = tree.get_metadata(id)
                                        .expect("metadata missing for instance present in tree");

                                    let instigating_path = match metadata.contributing_paths.get(0) {
                                        Some(path) => path,
                                        None => {
                                            log::warn!("Instance {} did not have an instigating path, but was considered for an update.", id);
                                            log::warn!("This is a Rojo bug. Please file an issue!");
                                            continue;
                                        }
                                    };

                                    let entry = imfs
                                        .get(instigating_path)
                                        .expect("could not get instigating path from filesystem");

                                    let snapshot = snapshot_from_imfs(&mut imfs, &entry)
                                        .expect("snapshot failed")
                                        .expect("snapshot did not return an instance");

                                    log::trace!("Computed snapshot: {:#?}", snapshot);

                                    let patch_set = compute_patch_set(&snapshot, &tree, id);
                                    let applied_patch_set = apply_patch_set(&mut tree, patch_set);

                                    log::trace!("Applied patch: {:#?}", applied_patch_set);

                                    applied_patches.push(applied_patch_set);
                                }
                            }
                            ImfsEvent::Removed(path) => {
                                log::warn!("TODO: Handle file remove events");
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
