use regex::Regex;
use serde_json;

use core::Route;
use plugin::{Plugin, PluginChain, TransformFileResult, RbxChangeResult, FileChangeResult};
use rbx::{RbxItem, RbxValue};
use vfs::VfsItem;

lazy_static! {
    static ref JSON_MODEL_PATTERN: Regex = Regex::new(r"^(.*?)\.model\.json$").unwrap();
}

pub struct JsonModelPlugin;

impl JsonModelPlugin {
    pub fn new() -> JsonModelPlugin {
        JsonModelPlugin
    }
}

impl Plugin for JsonModelPlugin {
    fn transform_file(&self, _plugins: &PluginChain, vfs_item: &VfsItem) -> TransformFileResult {
        match vfs_item {
            &VfsItem::File { ref contents, .. } => {
                let rbx_name = match JSON_MODEL_PATTERN.captures(vfs_item.name()) {
                    Some(captures) => captures.get(1).unwrap().as_str().to_string(),
                    None => return TransformFileResult::Pass,
                };

                let mut rbx_item: RbxItem = match serde_json::from_str(contents) {
                    Ok(v) => v,
                    Err(_) => {
                        eprintln!("Unable to parse JSON Model File named {}", vfs_item.name());

                        return TransformFileResult::Pass; // This should be an error in the future!
                    },
                };

                rbx_item.properties.insert("Name".to_string(), RbxValue::String {
                    value: rbx_name,
                });

                TransformFileResult::Value(Some(rbx_item))
            },
            &VfsItem::Dir { .. } => TransformFileResult::Pass,
        }
    }

    fn handle_file_change(&self, _route: &Route) -> FileChangeResult {
        FileChangeResult::Pass
    }

    fn handle_rbx_change(&self, _route: &Route, _rbx_item: &RbxItem) -> RbxChangeResult {
        RbxChangeResult::Pass
    }
}
