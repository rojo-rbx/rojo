use std::{fmt, fs, ops::Deref, path::Path};

use rlua::{Lua, RegistryKey};

use super::error::SnapshotError;

#[derive(Debug)]
pub struct InstanceSnapshotContext {
    /// Holds all the state needed to run user plugins as part of the snapshot
    /// process.
    ///
    /// If this is None, then plugins should not be evaluated at all.
    pub plugin_context: Option<SnapshotPluginContext>,
}

impl Default for InstanceSnapshotContext {
    fn default() -> Self {
        Self {
            plugin_context: None,
        }
    }
}

#[derive(Debug)]
pub struct SnapshotPluginContext {
    pub state: IgnoreDebug<Lua>,

    /// Registry keys pointing to the values returned by each user plugin. When
    /// processing user plugins, these should be applied in order.
    pub plugin_functions: Vec<RegistryKey>,
}

impl SnapshotPluginContext {
    pub fn new<P: AsRef<Path>>(plugin_paths: &[P]) -> Self {
        let lua_state = Lua::new();

        let plugin_functions = plugin_paths
            .iter()
            .map(|path| {
                let path = path.as_ref();

                let content =
                    fs::read_to_string(path).map_err(|err| SnapshotError::wrap(err, path))?;

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
            .collect::<Result<Vec<_>, SnapshotError>>()
            .expect("Plugin initialization error");

        Self {
            state: IgnoreDebug(lua_state),
            plugin_functions,
        }
    }
}

/// Utility type to enable having a field of a struct not implement Debug and
/// instead show a placeholder.
#[derive(Clone)]
pub struct IgnoreDebug<T>(pub T);

impl<T> fmt::Debug for IgnoreDebug<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "<no debug representation>")
    }
}

impl<T> Deref for IgnoreDebug<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

pub struct VfsSnapshotContext;
