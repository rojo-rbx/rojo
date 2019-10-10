use std::{fs, path::Path};

use rlua::{Lua, RegistryKey};

use crate::imfs::{Imfs, ImfsEntry, ImfsFetcher};

use super::{
    context::InstanceSnapshotContext,
    error::SnapshotError,
    middleware::{SnapshotInstanceResult, SnapshotMiddleware},
};

pub struct SnapshotUserPlugins;

impl SnapshotMiddleware for SnapshotUserPlugins {
    fn from_imfs<F: ImfsFetcher>(
        context: &InstanceSnapshotContext,
        _imfs: &mut Imfs<F>,
        _entry: &ImfsEntry,
    ) -> SnapshotInstanceResult<'static> {
        // User plugins are only enabled if present on the snapshot context.
        let plugin_context = match &context.plugin_context {
            Some(ctx) => ctx,
            None => return Ok(None),
        };

        // TODO: Store initialized plugins; this requires context to be mutable
        // and it isn't right now.
        initialize_plugins(&plugin_context.state, &plugin_context.plugin_paths)?;

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

            let content = fs::read_to_string(path).map_err(|err| SnapshotError::wrap(err, path))?;

            lua_state.context(|lua_context| {
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
