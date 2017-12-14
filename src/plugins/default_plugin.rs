use std::collections::HashMap;

use plugin::{Plugin, PluginResult};
use rbx::{RbxItem, RbxValue};
use vfs::VfsItem;

pub struct DefaultPlugin;

impl Plugin for DefaultPlugin {
    fn transform(item: &VfsItem) -> PluginResult {
        match item {
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
                // TODO: call back into plugin list and transform there instead

                let mut rbx_children = Vec::new();

                for (_, child_item) in children {
                    match Self::transform(child_item) {
                        PluginResult::Value(Some(rbx_item)) => {
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
