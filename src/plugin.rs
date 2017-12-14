use rbx::RbxItem;
use vfs::VfsItem;

pub enum PluginResult {
    Value(Option<RbxItem>),
    Pass,
}

pub trait Plugin {
    fn transform(item: &VfsItem) -> PluginResult;
}
