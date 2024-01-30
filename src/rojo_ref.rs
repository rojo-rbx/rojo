use std::sync::Arc;

use serde::{Deserialize, Serialize};

pub const REF_ID_ATTRIBUTE_NAME: &str = "Rojo_Id";
pub const REF_POINTER_ATTRIBUTE_PREFIX: &str = "Rojo_Target_";

// TODO add an internment strategy for RojoRefs
// Something like what rbx-dom does for SharedStrings probably works

#[derive(Debug, Default, PartialEq, Hash, Clone, Serialize, Deserialize, Eq)]
pub struct RojoRef(Arc<String>);

impl RojoRef {
    #[inline]
    pub fn new(id: String) -> Self {
        Self(Arc::from(id))
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
