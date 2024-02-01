use std::{fmt, sync::Arc};

use serde::{Deserialize, Serialize};

pub const REF_ID_ATTRIBUTE_NAME: &str = "Rojo_Id";
pub const REF_POINTER_ATTRIBUTE_PREFIX: &str = "Rojo_Target_";

// TODO add an internment strategy for RojoRefs
// Something like what rbx-dom does for SharedStrings probably works

#[derive(Debug, Default, PartialEq, Hash, Clone, Serialize, Deserialize, Eq)]
pub struct RojoRef(Arc<Vec<u8>>);

impl RojoRef {
    #[inline]
    pub fn new(id: Vec<u8>) -> Self {
        Self(Arc::from(id))
    }

    #[inline]
    pub fn from_string(id: String) -> Self {
        Self(Arc::from(id.into_bytes()))
    }

    #[inline]
    pub fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.0).ok()
    }
}

impl fmt::Display for RojoRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_str() {
            Some(str) => write!(f, "{str}"),
            None => {
                write!(f, "Binary({:?})", self.0.as_slice())
            }
        }
    }
}
