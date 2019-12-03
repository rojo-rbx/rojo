use rbx_dom_weak::{RbxId, RbxTree};

use crate::{
    snapshot::{InstanceContext, InstanceSnapshot},
    vfs::{Vfs, VfsEntry, VfsFetcher, VfsSnapshot},
};

use super::error::SnapshotError;

pub type SnapshotInstanceResult = Result<Option<InstanceSnapshot>, SnapshotError>;
pub type SnapshotFileResult = Option<(String, VfsSnapshot)>;

pub trait SnapshotMiddleware {
    fn from_vfs<F: VfsFetcher>(
        context: &InstanceContext,
        vfs: &Vfs<F>,
        entry: &VfsEntry,
    ) -> SnapshotInstanceResult;

    fn from_instance(_tree: &RbxTree, _id: RbxId) -> SnapshotFileResult {
        None
    }
}
