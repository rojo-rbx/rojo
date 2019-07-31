use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum ImfsSnapshot {
    File(FileSnapshot),
    Directory(DirectorySnapshot),
}

#[derive(Debug, Clone)]
pub struct FileSnapshot {
    pub contents: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct DirectorySnapshot {
    pub children: HashMap<String, ImfsSnapshot>,
}