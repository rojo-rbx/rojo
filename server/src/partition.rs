use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Partition {
    /// The path on the filesystem that this partition maps to.
    pub path: PathBuf,

    /// The route to the Roblox instance that this partition maps to.
    pub target: Vec<String>,
}
