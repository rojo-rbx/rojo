use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RbxItem {
    pub name: String,
    pub class_name: String,
    pub children: Vec<RbxItem>,
    pub properties: HashMap<String, RbxValue>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum RbxValue {
    String {
        value: String,
    },
}
