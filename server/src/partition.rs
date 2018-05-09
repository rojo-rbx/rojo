use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub struct Partition {
    /// The unique name of this partition, used for debugging.
    pub name: String,

    /// The path on the filesystem that this partition maps to.
    pub path: PathBuf,

    /// The route to the Roblox instance that this partition maps to.
    pub target: Vec<String>,
}
