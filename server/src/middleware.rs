use rbx::RbxInstance;
use vfs::VfsItem;

type Route = Vec<String>;

pub enum TransformFileResult {
    Value(Option<RbxInstance>),
    Pass,

    // TODO: Error case
}

pub enum RbxChangeResult {
    Write(Option<VfsItem>),
    Pass,

    // TODO: Error case
}

pub enum FileChangeResult {
    MarkChanged(Option<Vec<Route>>),
    Pass,
}

pub trait Middleware {
    /// Invoked when a file is read from the filesystem and needs to be turned
    /// into a Roblox instance.
    fn transform_file(&self, plugins: &MiddlewareChain, vfs_item: &VfsItem) -> TransformFileResult;

    /// Invoked when a Roblox Instance change is reported by the Roblox Studio
    /// plugin and needs to be turned into a file to save.
    fn handle_rbx_change(&self, route: &Route, rbx_item: &RbxInstance) -> RbxChangeResult;

    /// Invoked when a file changes on the filesystem. The result defines what
    /// routes are marked as needing to be refreshed.
    fn handle_file_change(&self, route: &Route) -> FileChangeResult;
}

/// A set of plugins that are composed in order.
pub struct MiddlewareChain {
    plugins: Vec<Box<Middleware + Send + Sync>>,
}

impl MiddlewareChain {
    pub fn new(plugins: Vec<Box<Middleware + Send + Sync>>) -> MiddlewareChain {
        MiddlewareChain {
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

    pub fn handle_rbx_change(&self, route: &Route, rbx_item: &RbxInstance) -> Option<VfsItem> {
        for plugin in &self.plugins {
            match plugin.handle_rbx_change(route, rbx_item) {
                RbxChangeResult::Write(vfs_item) => return vfs_item,
                RbxChangeResult::Pass => {},
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
