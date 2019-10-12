use std::path::PathBuf;

#[derive(Debug)]
pub enum VfsEvent {
    Modified(PathBuf),
    Created(PathBuf),
    Removed(PathBuf),
}
