use std::collections::HashMap;
use std::sync::{mpsc, RwLock, Mutex};

use id::{Id, get_id};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Message {
    InstanceChanged {
        id: Id,
    },
}

pub struct MessageSession {
   messages: RwLock<Vec<Message>>,
   message_listeners: Mutex<HashMap<Id, mpsc::Sender<()>>>,
}

impl MessageSession {
    pub fn new() -> MessageSession {
        MessageSession {
            messages: RwLock::new(Vec::new()),
            message_listeners: Mutex::new(HashMap::new()),
        }
    }

    pub fn push_messages(&self, new_messages: &[Message]) {
        let message_listeners = self.message_listeners.lock().unwrap();

        {
            let mut messages = self.messages.write().unwrap();
            messages.extend_from_slice(new_messages);
        }

        {
            for listener in message_listeners.values() {
                listener.send(()).unwrap();
            }
        }
    }

    pub fn subscribe(&self, sender: mpsc::Sender<()>) -> Id {
        let id = get_id();

        {
            let mut message_listeners = self.message_listeners.lock().unwrap();
            message_listeners.insert(id, sender);
        }

        id
    }

    pub fn unsubscribe(&self, id: Id) {
        {
            let mut message_listeners = self.message_listeners.lock().unwrap();
            message_listeners.remove(&id);
        }
    }

    pub fn get_message_cursor(&self) -> u32 {
        self.messages.read().unwrap().len() as u32
    }

    pub fn get_messages_since(&self, cursor: u32) -> (u32, Vec<Message>) {
        let messages = self.messages.read().unwrap();

        let current_cursor = messages.len() as u32;

        // Cursor is out of bounds
        if cursor > current_cursor {
            return (current_cursor, Vec::new());
        }

        // No new messages
        if cursor == current_cursor {
            return (current_cursor, Vec::new());
        }

        (current_cursor, messages[(cursor as usize)..].to_vec())
    }
}