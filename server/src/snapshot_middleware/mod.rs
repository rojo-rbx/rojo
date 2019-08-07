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

macro_rules! middlewares {
    ( $($middleware: ident,)* ) => {
        /// Generates a snapshot of instances from the given ImfsEntry.
        pub fn snapshot_from_imfs<F: ImfsFetcher>(
            imfs: &mut Imfs<F>,
            entry: &ImfsEntry,
        ) -> SnapshotInstanceResult<'static> {
            $(
                if let Some(snapshot) = $middleware::from_imfs(imfs, entry)? {
                    return Ok(Some(snapshot));
                }
            )*

            Ok(None)
        }

        /// Generates an in-memory filesystem snapshot of the given Roblox
        /// instance.
        pub fn snapshot_from_instance(tree: &RbxTree, id: RbxId) -> SnapshotFileResult {
            $(
                if let Some(result) = $middleware::from_instance(tree, id) {
                    return Some(result);
                }
            )*

            None
        }
    };
}

middlewares! {
    SnapshotProject,
    // SnapshotJsonModel,
    // SnapshotRbxmx,
    // SnapshotRbxm,
    // SnapshotScript,
    // SnapshotCsv,
    SnapshotTxt,
    SnapshotDir,
}