use std::collections::HashMap;

/// Represents data about a Roblox instance
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RbxInstance {
    pub name: String,
    pub class_name: String,
    pub children: Vec<RbxInstance>,
    pub properties: HashMap<String, RbxValue>,

    /// The route that this instance was generated from, if there was one.
    pub route: Option<Vec<String>>,
}

/// Any kind value that can be used by Roblox
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum RbxValue {
    String {
        value: String,
    },
    Bool {
        value: bool,
    },
    Number {
        value: f64,
    },

    // TODO: Compound types like Vector3
}
