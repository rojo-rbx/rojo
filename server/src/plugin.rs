use rbx::RbxInstance;
use vfs::VfsItem;
use core::Route;

pub enum TransformFileResult {
    Value(Option<RbxInstance>),
    Pass,

    // TODO: Error case
}

pub enum FileChangeResult {
    MarkChanged(Option<Vec<Route>>),
    Pass,
}

pub trait Plugin {
    /// Invoked when a file is read from the filesystem and needs to be turned
    /// into a Roblox instance.
    fn transform_file(&self, plugins: &PluginChain, vfs_item: &VfsItem) -> TransformFileResult;

    /// Invoked when a file changes on the filesystem. The result defines what
    /// routes are marked as needing to be refreshed.
    fn handle_file_change(&self, route: &Route) -> FileChangeResult;
}

/// A set of plugins that are composed in order.
pub struct PluginChain {
    plugins: Vec<Box<Plugin + Send + Sync>>,
}

impl PluginChain {
    pub fn new(plugins: Vec<Box<Plugin + Send + Sync>>) -> PluginChain {
        PluginChain {
            plugins,
        }
    }

    pub fn transform_file(&self, vfs_item: &VfsItem) -> Option<RbxInstance> {
        for plugin in &self.plugins {
            match plugin.transform_file(self, vfs_item) {
                TransformFileResult::Value(rbx_item) => return rbx_item,
                TransformFileResult::Pass => {},
            }
        }

        None
    }

    pub fn handle_file_change(&self, route: &Route) -> Option<Vec<Route>> {
        for plugin in &self.plugins {
            match plugin.handle_file_change(route) {
                FileChangeResult::MarkChanged(changes) => return changes,
                FileChangeResult::Pass => {},
            }
        }

        None
    }
}
