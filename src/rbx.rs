use std::collections::HashMap;

/// Represents data about a Roblox instance
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RbxInstance {
    pub name: String,
    pub class_name: String,
    pub children: Vec<RbxInstance>,
    pub properties: HashMap<String, RbxValue>,
}

/// Any kind value that can be used by Roblox
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum RbxValue {
    String {
        value: String,
    },

    // TODO: Other primitives
    // TODO: Compound types like Vector3
}
