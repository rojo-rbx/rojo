use std::sync::Arc;

use serde::{Deserialize, Serialize};

pub const REF_ID_ATTRIBUTE_NAME: &str = "Rojo_Id";
pub const REF_POINTER_ATTRIBUTE_PREFIX: &str = "Rojo_Target_";

// TODO add an internment strategy for RojoRefs
// Something like what rbx-dom does for SharedStrings probably works

#[derive(Debug, Default, PartialEq, Hash, Clone, Serialize, Deserialize, Eq)]
pub struct RojoRef(Option<Arc<str>>);

impl RojoRef {
    #[inline]
    pub fn none() -> Self {
        Self(None)
    }

    #[inline]
    pub fn some(id: String) -> Self {
        Self(Some(Arc::from(id)))
    }

    #[inline]
    pub fn is_some(&self) -> bool {
        self.0.is_some()
    }
}

impl From<Option<String>> for RojoRef {
    fn from(value: Option<String>) -> Self {
        Self(value.map(Arc::from))
    }
}

impl From<Arc<str>> for RojoRef {
    fn from(value: Arc<str>) -> Self {
        Self(Some(value))
    }
}
