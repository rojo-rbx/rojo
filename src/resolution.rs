use rbx_dom_weak::types::Variant;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UnresolvedValue {
    FullyQualified(Variant),
    PartiallyQualified(PartiallyQualifiedValue),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PartiallyQualifiedValue {
    String(String),
    Array2([f32; 2]),
    Array3([f32; 3]),
    Array4([f32; 4]),
}

impl UnresolvedValue {
    pub fn resolve(self, class_name: &str, prop_name: &str) -> Variant {
        unimplemented!()
    }
}
