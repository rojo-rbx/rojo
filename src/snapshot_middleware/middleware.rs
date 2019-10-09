use std::path::{Path, PathBuf};

use rbx_dom_weak::{RbxId, RbxTree};

use crate::{
    imfs::{Imfs, ImfsEntry, ImfsFetcher, ImfsSnapshot},
    snapshot::InstanceSnapshot,
};

use super::error::SnapshotError;

pub type SnapshotInstanceResult<'a> = Result<Option<InstanceSnapshot<'a>>, SnapshotError>;
pub type SnapshotFileResult = Option<(String, ImfsSnapshot)>;

pub trait SnapshotMiddleware {
    fn from_imfs<F: ImfsFetcher>(
        imfs: &mut Imfs<F>,
        entry: &ImfsEntry,
    ) -> SnapshotInstanceResult<'static>;

    fn from_instance(tree: &RbxTree, id: RbxId) -> SnapshotFileResult;

    fn change_affects_paths(path: &Path) -> Vec<PathBuf> {
        vec![path.to_path_buf()]
    }
}
