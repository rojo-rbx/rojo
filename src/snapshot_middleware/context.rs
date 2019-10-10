use std::fmt;

use rlua::Lua;

#[derive(Debug)]
pub struct InstanceSnapshotContext {
    /// Empty struct that will be used later to fill out required Lua state for
    /// user plugins.
    pub plugin_context: Option<SnapshotPluginContext>,
}

impl Default for InstanceSnapshotContext {
    fn default() -> Self {
        Self {
            plugin_context: None,
        }
    }
}

pub struct SnapshotPluginContext {
    state: Lua,
}

impl fmt::Debug for SnapshotPluginContext {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "SnapshotPluginContext")
    }
}

pub struct ImfsSnapshotContext;
