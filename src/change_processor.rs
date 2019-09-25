//! Defines the process by which changes are pulled from the Imfs, filtered, and
//! used to mutate Rojo's tree during a live session.
//!
//! This object is owned by a ServeSession.

use std::sync::{Arc, Mutex};

use crossbeam_channel::{select, Receiver, Sender};
use jod_thread::JoinHandle;

use crate::{
    imfs::{Imfs, ImfsFetcher},
    message_queue::MessageQueue,
    snapshot::{AppliedPatchSet, RojoTree},
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
            .spawn(move || Self::main_task(shutdown_receiver, tree, message_queue, imfs))
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
        log::trace!("ChangeProcessor thread started");

        let imfs_receiver = {
            let imfs = imfs.lock().unwrap();
            imfs.change_receiver()
        };

        loop {
            select! {
                recv(imfs_receiver) -> event => {
                    let event = event.unwrap();

                    log::trace!("Imfs event: {:?}", event);

                    {
                        let mut imfs = imfs.lock().unwrap();
                        imfs.commit_change(&event).expect("Error applying IMFS change");
                    }

                    let patch_set = {
                        let _tree = tree.lock().unwrap();

                        // TODO: Apply changes to tree based on IMFS and
                        // calculate applied patch set from it.
                        AppliedPatchSet::new()
                    };

                    {
                        message_queue.push_messages(&[patch_set]);
                    }
                },
                recv(shutdown_receiver) -> _ => {
                    break;
                },
            }
        }

        log::trace!("ChangeProcessor thread stopping");
    }
}

impl Drop for ChangeProcessor {
    fn drop(&mut self) {
        let _ = self.shutdown_sender.send(());
    }
}
