use rbx::RbxItem;
use vfs::VfsItem;

pub enum PluginResult {
    Value(Option<RbxItem>),
    Pass,
}

pub trait Plugin {
    fn transform_file(&self, plugins: &PluginChain, vfs_item: &VfsItem) -> PluginResult;
}

pub struct PluginChain {
    plugins: Vec<Box<Plugin + Send>>,
}

impl PluginChain {
    pub fn new(plugins: Vec<Box<Plugin + Send>>) -> PluginChain {
        PluginChain {
            plugins,
        }
    }

    pub fn transform_file(&self, vfs_item: &VfsItem) -> Option<RbxItem> {
        for plugin in &self.plugins {
            match plugin.transform_file(self, vfs_item) {
                PluginResult::Value(rbx_item) => return rbx_item,
                PluginResult::Pass => {},
            }
        }

        None
    }
}
