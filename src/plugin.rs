use rbx::RbxItem;
use vfs::VfsItem;
use core::Route;

pub enum TransformFileResult {
    Value(Option<RbxItem>),
    Pass,
}

pub enum FileChangeResult {
    MarkChanged(Option<Vec<Route>>),
    Pass,
}

pub trait Plugin {
    fn transform_file(&self, plugins: &PluginChain, vfs_item: &VfsItem) -> TransformFileResult;
    fn handle_file_change(&self, route: &Route) -> FileChangeResult;
}

pub struct PluginChain {
    plugins: Vec<Box<Plugin + Send + Sync>>,
}

impl PluginChain {
    pub fn new(plugins: Vec<Box<Plugin + Send + Sync>>) -> PluginChain {
        PluginChain {
            plugins,
        }
    }

    pub fn transform_file(&self, vfs_item: &VfsItem) -> Option<RbxItem> {
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
