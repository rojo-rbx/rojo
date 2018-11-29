use serde_derive::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(Uuid);

impl SessionId {
    pub fn new() -> SessionId {
        SessionId(Uuid::new_v4())
    }
}