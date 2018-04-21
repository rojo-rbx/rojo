use std::collections::HashMap;

/// A VfsItem represents either a file or directory as it came from the filesystem.
///
/// The interface here is intentionally simplified to make it easier to traverse
/// files that have been read into memory.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum VfsItem {
    File {
        route: Vec<String>,
        file_name: String,
        contents: String,
    },
    Dir {
        route: Vec<String>,
        file_name: String,
        children: HashMap<String, VfsItem>,
    },
}

impl VfsItem {
    pub fn name(&self) -> &String {
        match self {
            &VfsItem::File { ref file_name , .. } => file_name,
            &VfsItem::Dir { ref file_name , .. } => file_name,
        }
    }

    pub fn route(&self) -> &[String] {
        match self {
            &VfsItem::File { ref route, .. } => route,
            &VfsItem::Dir { ref route, .. } => route,
        }
    }
}
