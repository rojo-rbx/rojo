//! Defines the semantics that Rojo uses to turn entries on the filesystem into
//! Roblox instances using the instance snapshot subsystem.

#![allow(dead_code)]

mod context;
mod dir;
mod error;
mod middleware;
mod project;
mod txt;

use rbx_dom_weak::{RbxTree, RbxId};

use crate::imfs::new::{Imfs, ImfsEntry, ImfsFetcher};
use self::{
    middleware::{SnapshotInstanceResult, SnapshotFileResult, SnapshotMiddleware},
    project::SnapshotProject,
    txt::SnapshotTxt,
    dir::SnapshotDir,
};

/// Placeholder function for stubbing out snapshot middleware
pub fn snapshot_from_imfs<F: ImfsFetcher>(imfs: &mut Imfs<F>, entry: &ImfsEntry) -> SnapshotInstanceResult<'static> {
    if let Some(snapshot) = SnapshotProject::from_imfs(imfs, entry)? {
        Ok(Some(snapshot))
    } else if let Some(snapshot) = SnapshotTxt::from_imfs(imfs, entry)? {
        Ok(Some(snapshot))
    } else if let Some(snapshot) = SnapshotDir::from_imfs(imfs, entry)? {
        Ok(Some(snapshot))
    } else {
        Ok(None)
    }
}

/// Placeholder function for stubbing out snapshot middleware
pub fn snapshot_from_instance(_tree: &RbxTree, _id: RbxId) -> SnapshotFileResult {
    unimplemented!();
}