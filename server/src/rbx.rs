use std::collections::HashMap;

use id::Id;

// TODO: Switch to enum to represent more value types
pub type RbxValue = String;

#[derive(Debug, Clone)]
pub struct RbxInstance {
    /// Maps to the `Name` property on Instance.
    pub name: String,

    /// Maps to the `ClassName` property on Instance.
    pub class_name: String,

    /// Maps to the `Parent` property on Instance.
    pub parent: Option<Id>,

    /// Contains all other properties of an Instance.
    pub properties: HashMap<String, RbxValue>,
}
