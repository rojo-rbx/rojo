use std::collections::HashMap;

use regex::Regex;

use plugin::{Plugin, PluginChain, PluginResult};
use rbx::{RbxItem, RbxValue};
use vfs::VfsItem;

lazy_static! {
    static ref SERVER_PATTERN: Regex = Regex::new(r"^(.*?)\.server\.lua$").unwrap();
    static ref CLIENT_PATTERN: Regex = Regex::new(r"^(.*?)\.client\.lua$").unwrap();
    static ref MODULE_PATTERN: Regex = Regex::new(r"^(.*?)\.lua$").unwrap();
}

pub struct ScriptPlugin;

impl ScriptPlugin {
    pub fn new() -> ScriptPlugin {
        ScriptPlugin
    }
}

impl Plugin for ScriptPlugin {
    fn transform_file(&self, plugins: &PluginChain, vfs_item: &VfsItem) -> PluginResult {
        match vfs_item {
            &VfsItem::File { ref contents, ref name } => {
                let (class_name, rbx_name) = {
                    if let Some(captures) = SERVER_PATTERN.captures(name) {
                        ("Script".to_string(), captures.get(1).unwrap().as_str().to_string())
                    } else if let Some(captures) = CLIENT_PATTERN.captures(name) {
                        ("LocalScript".to_string(), captures.get(1).unwrap().as_str().to_string())
                    } else if let Some(captures) = MODULE_PATTERN.captures(name) {
                        ("ModuleScript".to_string(), captures.get(1).unwrap().as_str().to_string())
                    } else {
                        return PluginResult::Pass;
                    }
                };

                let mut properties = HashMap::new();

                properties.insert("Source".to_string(), RbxValue::String {
                    value: contents.clone(),
                });

                PluginResult::Value(Some(RbxItem {
                    name: rbx_name,
                    class_name: class_name,
                    children: Vec::new(),
                    properties,
                }))
            },
            &VfsItem::Dir { ref children, ref name } => {
                PluginResult::Pass
            },
        }
    }
}
