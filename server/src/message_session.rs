use std::collections::HashMap;
use std::sync::{mpsc, Arc, RwLock, Mutex};

use id::{Id, get_id};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Message {
    InstanceChanged {
        id: Id,
    },
}

#[derive(Clone)]
pub struct MessageSession {
   pub messages: Arc<RwLock<Vec<Message>>>,
   pub message_listeners: Arc<Mutex<HashMap<Id, mpsc::Sender<()>>>>,
}

impl MessageSession {
    pub fn new() -> MessageSession {
        MessageSession {
            messages: Arc::new(RwLock::new(Vec::new())),
            message_listeners: Arc::new(Mutex::new(HashMap::new())),
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

    pub fn get_message_cursor(&self) -> i32 {
        self.messages.read().unwrap().len() as i32 - 1
    }
}
