#[derive(Debug)]
pub struct InstanceSnapshotContext {
    /// Empty struct that will be used later to fill out required Lua state for
    /// user plugins.
    pub plugin_context: Option<()>,
}

impl Default for InstanceSnapshotContext {
    fn default() -> Self {
        Self {
            plugin_context: None,
        }
    }
}

pub struct ImfsSnapshotContext;
