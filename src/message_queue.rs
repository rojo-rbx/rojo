use std::sync::{Mutex, RwLock};

use futures::channel::oneshot;

/// A message queue with persistent history that can be subscribed to.
///
/// Definitely non-optimal. This would ideally be a lockless mpmc queue.
#[derive(Default)]
pub struct MessageQueue<T> {
    messages: RwLock<Vec<T>>,
    message_listeners: Mutex<Vec<Listener<T>>>,
}

impl<T: Clone> MessageQueue<T> {
    pub fn new() -> MessageQueue<T> {
        MessageQueue {
            messages: RwLock::new(Vec::new()),
            message_listeners: Mutex::new(Vec::new()),
        }
    }

    pub fn push_messages(&self, new_messages: &[T]) {
        let mut message_listeners = self.message_listeners.lock().unwrap();
        let mut messages = self.messages.write().unwrap();
        messages.extend_from_slice(new_messages);

        let mut remaining_listeners = Vec::new();

        for listener in message_listeners.drain(..) {
            match fire_listener_if_ready(&messages, listener) {
                Ok(_) => {}
                Err(listener) => remaining_listeners.push(listener),
            }
        }

        // Without this annotation, Rust gets confused since the first argument
        // is a MutexGuard, but the second is a Vec.
        *message_listeners = remaining_listeners;
    }

    /// Subscribe to any messages occurring after the given message cursor.
    pub fn subscribe(&self, cursor: u32) -> oneshot::Receiver<(u32, Vec<T>)> {
        let (sender, receiver) = oneshot::channel();

        let listener = {
            let listener = Listener { sender, cursor };

            let messages = self.messages.read().unwrap();

            match fire_listener_if_ready(&messages, listener) {
                Ok(_) => return receiver,
                Err(listener) => listener,
            }
        };

        let mut message_listeners = self.message_listeners.lock().unwrap();
        message_listeners.push(listener);

        receiver
    }

    /// Subscribe to any messages being pushed into the queue.
    ///
    /// This method is only useful in tests. Non-test code should use subscribe
    /// instead.
    #[cfg(test)]
    #[allow(unused)]
    pub fn subscribe_any(&self) -> oneshot::Receiver<(u32, Vec<T>)> {
        let cursor = {
            let messages = self.messages.read().unwrap();
            messages.len() as u32
        };

        self.subscribe(cursor)
    }

    pub fn cursor(&self) -> u32 {
        self.messages.read().unwrap().len() as u32
    }
}

struct Listener<T> {
    sender: oneshot::Sender<(u32, Vec<T>)>,
    cursor: u32,
}

fn fire_listener_if_ready<T: Clone>(
    messages: &[T],
    listener: Listener<T>,
) -> Result<(), Listener<T>> {
    let current_cursor = messages.len() as u32;

    if listener.cursor < current_cursor {
        let new_messages = messages[(listener.cursor as usize)..].to_vec();
        let _ = listener.sender.send((current_cursor, new_messages));
        Ok(())
    } else {
        Err(listener)
    }
}
