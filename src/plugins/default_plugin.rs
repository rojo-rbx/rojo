use std::collections::HashMap;

use plugin::{Plugin, PluginChain, PluginResult};
use rbx::{RbxItem, RbxValue};
use vfs::VfsItem;

pub struct DefaultPlugin;

impl DefaultPlugin {
    pub fn new() -> DefaultPlugin {
        DefaultPlugin
    }
}

impl Plugin for DefaultPlugin {
    fn transform_file(&self, plugins: &PluginChain, vfs_item: &VfsItem) -> PluginResult {
        match vfs_item {
            &VfsItem::File { ref contents, ref name } => {
                let mut properties = HashMap::new();

                properties.insert("Value".to_string(), RbxValue::String {
                    value: contents.clone(),
                });

                PluginResult::Value(Some(RbxItem {
                    name: name.clone(),
                    class_name: "StringValue".to_string(),
                    children: Vec::new(),
                    properties,
                }))
            },
            &VfsItem::Dir { ref children, ref name } => {
                let mut rbx_children = Vec::new();

                for (_, child_item) in children {
                    match plugins.transform_file(child_item) {
                        Some(rbx_item) => {
                            rbx_children.push(rbx_item);
                        },
                        _ => {},
                    }
                }

                PluginResult::Value(Some(RbxItem {
                    name: name.clone(),
                    class_name: "Folder".to_string(),
                    children: rbx_children,
                    properties: HashMap::new(),
                }))
            },
        }
    }
}
