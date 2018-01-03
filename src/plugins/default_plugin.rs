use std::collections::HashMap;

use core::Route;
use plugin::{Plugin, PluginChain, TransformFileResult, RbxChangeResult, FileChangeResult};
use rbx::{RbxItem, RbxValue};
use vfs::VfsItem;

/// A plugin with simple transforms:
/// * Directories become Folder instances
/// * Files become StringValue objects with 'Value' as their contents
pub struct DefaultPlugin;

impl DefaultPlugin {
    pub fn new() -> DefaultPlugin {
        DefaultPlugin
    }
}

impl Plugin for DefaultPlugin {
    fn transform_file(&self, plugins: &PluginChain, vfs_item: &VfsItem) -> TransformFileResult {
        match vfs_item {
            &VfsItem::File { ref contents, .. } => {
                let mut properties = HashMap::new();

                properties.insert("Value".to_string(), RbxValue::String {
                    value: contents.clone(),
                });

                TransformFileResult::Value(Some(RbxItem {
                    name: vfs_item.name().clone(),
                    class_name: "StringValue".to_string(),
                    children: Vec::new(),
                    properties,
                }))
            },
            &VfsItem::Dir { ref children, .. } => {
                let mut rbx_children = Vec::new();

                for (_, child_item) in children {
                    match plugins.transform_file(child_item) {
                        Some(rbx_item) => {
                            rbx_children.push(rbx_item);
                        },
                        _ => {},
                    }
                }

                TransformFileResult::Value(Some(RbxItem {
                    name: vfs_item.name().clone(),
                    class_name: "Folder".to_string(),
                    children: rbx_children,
                    properties: HashMap::new(),
                }))
            },
        }
    }

    fn handle_file_change(&self, route: &Route) -> FileChangeResult {
        FileChangeResult::MarkChanged(Some(vec![route.clone()]))
    }

    fn handle_rbx_change(&self, _route: &Route, _rbx_item: &RbxItem) -> RbxChangeResult {
        RbxChangeResult::Pass
    }
}
