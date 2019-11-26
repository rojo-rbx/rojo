use crate::vfs::{Vfs, VfsEntry, VfsFetcher};

use super::{
    context::InstanceSnapshotContext,
    middleware::{SnapshotInstanceResult, SnapshotMiddleware},
};

/// Handles snapshotting of any file that a user plugin wants to handle.
///
/// User plugins are specified in the project file, but there are never user
/// plugins specified unless a Cargo feature is enabled, `user-plugins`.
/// Additionally, extra data needs to be set up inside the snapshot context
/// which is not currently wired up.
pub struct SnapshotUserPlugins;

impl SnapshotMiddleware for SnapshotUserPlugins {
    fn from_vfs<F: VfsFetcher>(
        _context: &mut InstanceSnapshotContext,
        _vfs: &Vfs<F>,
        _entry: &VfsEntry,
    ) -> SnapshotInstanceResult {
        // TODO: Invoke plugin here and get result out.

        // The current plan for plugins here is to make them work
        // like Redux/Rodux middleware. A plugin will be a function
        // that accepts the next middleware in the chain as a
        // function and the snapshot subject (the VFS entry).
        //
        // Plugins can (but don't have to) invoke the next snapshot
        // function and may or may not mutate the result. The hope
        // is that this model enables the most flexibility possible
        // for plugins to modify existing Rojo output, as well as
        // generate new outputs.
        //
        // Open questions:
        // * How will middleware be ordered? Does putting user
        //   middleware always at the beginning or always at the end
        //   of the chain reduce the scope of what that middleware
        //   can do?
        //
        // * Will plugins hurt Rojo's ability to parallelize
        //   snapshotting in the future?
        //
        // * Do the mutable handles to the Vfs and the snapshot
        //   context prevent plugins from invoking other plugins
        //   indirectly?
        //
        // * Will there be problems using a single Lua state because
        //   of re-entrancy?
        //
        // * Can the Lua <-> Rojo bindings used for middleware be
        //   reused for or from another project like Remodel?

        Ok(None)
    }
}
