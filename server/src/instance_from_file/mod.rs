//! Defines the semantics that Rojo uses to turn entries on the filesystem into
//! Roblox instances using the instance snapshot subsystem.

#![allow(dead_code)]

mod context;
mod error;
mod from_imfs;
mod middleware;
mod project;
mod txt;

use rbx_dom_weak::{RbxTree, RbxId};

use crate::imfs::new::{Imfs, ImfsEntry, ImfsFetcher};
use self::middleware::{SnapshotInstanceResult, SnapshotFileResult};

/// Placeholder function for stubbing out snapshot middleware
pub fn snapshot_from_imfs<F: ImfsFetcher>(_imfs: &mut Imfs<F>, _entry: &ImfsEntry) -> SnapshotInstanceResult<'static> {
    unimplemented!();
}

/// Placeholder function for stubbing out snapshot middleware
pub fn snapshot_from_instance(_tree: &RbxTree, _id: RbxId) -> SnapshotFileResult {
    unimplemented!();
}