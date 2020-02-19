use std::path::Path;

use vfs::Vfs;

use crate::snapshot::{InstanceContext, InstanceSnapshot};

use super::error::SnapshotError;

pub type SnapshotInstanceResult = Result<Option<InstanceSnapshot>, SnapshotError>;

pub trait SnapshotMiddleware {
    fn from_vfs(context: &InstanceContext, vfs: &Vfs, path: &Path) -> SnapshotInstanceResult;
}
