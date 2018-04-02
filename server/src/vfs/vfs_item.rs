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
        contents: String,
    },
    Dir {
        route: Vec<String>,
        children: HashMap<String, VfsItem>,
    },
}

impl VfsItem {
    pub fn name(&self) -> &String {
        self.route().last().unwrap()
    }

    pub fn route(&self) -> &[String] {
        match self {
            &VfsItem::File { ref route, .. } => route,
            &VfsItem::Dir { ref route, .. } => route,
        }
    }
}
