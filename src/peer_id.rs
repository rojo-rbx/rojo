use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Uniquely identifies a client or server during a serve session.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerId(Uuid);

impl PeerId {
    pub fn new() -> PeerId {
        PeerId(Uuid::new_v4())
    }
}

impl fmt::Display for PeerId {
    fn fmt(&self, writer: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(writer, "{}", self.0)
    }
}
