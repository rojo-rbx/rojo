use std::{fs, path::Path};

use rlua::{Lua, RegistryKey};

use crate::imfs::{Imfs, ImfsEntry, ImfsFetcher};

use super::{
    context::InstanceSnapshotContext,
    error::SnapshotError,
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
    fn from_imfs<F: ImfsFetcher>(
        context: &mut InstanceSnapshotContext,
        _imfs: &mut Imfs<F>,
        _entry: &ImfsEntry,
    ) -> SnapshotInstanceResult<'static> {
        // User plugins are only enabled if present on the snapshot context.
        let plugin_context = match &mut context.plugin_context {
            Some(ctx) => ctx,
            None => return Ok(None),
        };

        // If the plugins listed for use haven't been loaded yet, read them into
        // memory, run them, and keep the result they return as a registry key
        // into our Lua state.
        let keys = match &plugin_context.plugin_functions {
            Some(keys) => keys,
            None => {
                plugin_context.plugin_functions = Some(initialize_plugins(
                    &plugin_context.state,
                    &plugin_context.plugin_paths,
                )?);
                plugin_context.plugin_functions.as_ref().unwrap()
            }
        };

        plugin_context.state.context(|lua_context| {
            lua_context.scope(|_scope| {
                for _key in keys {
                    // TODO: Invoke plugin here and get result out.

                    // The current plan for plugins here is to make them work
                    // like Redux/Rodux middleware. A plugin will be a function
                    // that accepts the next middleware in the chain as a
                    // function and the snapshot subject (the IMFS entry).
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
                    // * Do the mutable handles to the Imfs and the snapshot
                    //   context prevent plugins from invoking other plugins
                    //   indirectly?
                    //
                    // * Will there be problems using a single Lua state because
                    //   of re-entrancy?
                    //
                    // * Can the Lua <-> Rojo bindings used for middleware be
                    //   reused for or from another project like Remodel?
                }
            })
        });

        Ok(None)
    }
}

fn initialize_plugins<P: AsRef<Path>>(
    lua_state: &Lua,
    plugin_paths: &[P],
) -> Result<Vec<RegistryKey>, SnapshotError> {
    plugin_paths
        .iter()
        .map(|path| {
            let path = path.as_ref();

            // TODO: This path is currently relative to the working directory,
            // but should be relative to the folder containing the project file.
            let content = fs::read_to_string(path).map_err(|err| SnapshotError::wrap(err, path))?;

            lua_state.context(|lua_context| {
                // Plugins are currently expected to return a function that will
                // be run when a snapshot needs to be generated.
                let result = lua_context
                    .load(&content)
                    .set_name(&path.display().to_string())?
                    .call::<_, rlua::Function>(())?;

                let key = lua_context.create_registry_value(result)?;

                Ok(key)
            })
        })
        .collect::<Result<Vec<_>, _>>()
}
