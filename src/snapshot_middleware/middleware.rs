use rbx_dom_weak::{RbxId, RbxTree};

use crate::{
    snapshot::InstanceSnapshot,
    vfs::{Vfs, VfsEntry, VfsFetcher, VfsSnapshot},
};

use super::{context::InstanceSnapshotContext, error::SnapshotError};

pub type SnapshotInstanceResult<'a> = Result<Option<InstanceSnapshot<'a>>, SnapshotError>;
pub type SnapshotFileResult = Option<(String, VfsSnapshot)>;

pub trait SnapshotMiddleware {
    fn from_vfs<F: VfsFetcher>(
        context: &mut InstanceSnapshotContext,
        vfs: &Vfs<F>,
        entry: &VfsEntry,
    ) -> SnapshotInstanceResult<'static>;

    fn from_instance(_tree: &RbxTree, _id: RbxId) -> SnapshotFileResult {
        None
    }
}
