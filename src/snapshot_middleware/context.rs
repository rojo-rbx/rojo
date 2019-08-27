pub struct InstanceSnapshotContext {
    /// Empty struct that will be used later to fill out required Lua state for
    /// user plugins.
    pub plugin_context: Option<()>,
}

pub struct ImfsSnapshotContext;
