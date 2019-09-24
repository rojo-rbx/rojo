//! Defines the semantics that Rojo uses to turn entries on the filesystem into
//! Roblox instances using the instance snapshot subsystem.

#![allow(dead_code)]

mod context;
mod csv;
mod dir;
mod error;
mod json_model;
mod lua;
mod middleware;
mod project;
mod rbxlx;
mod rbxm;
mod rbxmx;
mod txt;

use rbx_dom_weak::{RbxId, RbxTree};

use self::{
    csv::SnapshotCsv,
    dir::SnapshotDir,
    json_model::SnapshotJsonModel,
    lua::SnapshotLua,
    middleware::{SnapshotFileResult, SnapshotInstanceResult, SnapshotMiddleware},
    project::SnapshotProject,
    rbxlx::SnapshotRbxlx,
    rbxm::SnapshotRbxm,
    rbxmx::SnapshotRbxmx,
    txt::SnapshotTxt,
};
use crate::imfs::{Imfs, ImfsEntry, ImfsFetcher};

macro_rules! middlewares {
    ( $($middleware: ident,)* ) => {
        /// Generates a snapshot of instances from the given ImfsEntry.
        pub fn snapshot_from_imfs<F: ImfsFetcher>(
            imfs: &mut Imfs<F>,
            entry: &ImfsEntry,
        ) -> SnapshotInstanceResult<'static> {
            $(
                log::trace!("trying middleware {} on {}", stringify!($middleware), entry.path().display());

                if let Some(snapshot) = $middleware::from_imfs(imfs, entry)? {
                    log::trace!("middleware {} success on {}", stringify!($middleware), entry.path().display());
                    return Ok(Some(snapshot));
                }
            )*

            log::trace!("no middleware returned Ok(Some)");
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
    SnapshotJsonModel,
    SnapshotRbxlx,
    SnapshotRbxmx,
    SnapshotRbxm,
    SnapshotLua,
    SnapshotCsv,
    SnapshotTxt,
    SnapshotDir,
}
