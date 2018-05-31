use std::collections::HashMap;

use id::Id;

// TODO: Switch to enum to represent more value types
pub type RbxValue = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RbxInstance {
    /// Maps to the `Name` property on Instance.
    pub name: String,

    /// Maps to the `ClassName` property on Instance.
    pub class_name: String,

    /// Contains all other properties of an Instance.
    pub properties: HashMap<String, RbxValue>,

    /// All of the children of this instance. Order is relevant to preserve!
    pub children: Vec<Id>,
}
