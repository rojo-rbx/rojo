use std::path::PathBuf;

#[derive(Debug)]
pub enum ImfsEvent {
    Modified(PathBuf),
    Created(PathBuf),
    Removed(PathBuf),
}
