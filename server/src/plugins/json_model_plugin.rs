use regex::Regex;
use serde_json;

use core::Route;
use plugin::{Plugin, PluginChain, TransformFileResult, FileChangeResult};
use rbx::RbxInstance;
use vfs::VfsItem;

lazy_static! {
    static ref JSON_MODEL_PATTERN: Regex = Regex::new(r"^(.*?)\.model\.json$").unwrap();
}

static JSON_MODEL_INIT: &'static str = "init.model.json";

pub struct JsonModelPlugin;

impl JsonModelPlugin {
    pub fn new() -> JsonModelPlugin {
        JsonModelPlugin
    }
}

impl Plugin for JsonModelPlugin {
    fn transform_file(&self, plugins: &PluginChain, vfs_item: &VfsItem) -> TransformFileResult {
        match vfs_item {
            &VfsItem::File { ref contents, .. } => {
                let rbx_name = match JSON_MODEL_PATTERN.captures(vfs_item.name()) {
                    Some(captures) => captures.get(1).unwrap().as_str().to_string(),
                    None => return TransformFileResult::Pass,
                };

                let mut rbx_item: RbxInstance = match serde_json::from_str(contents) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("Unable to parse JSON Model File named {}: {}", vfs_item.name(), e);

                        return TransformFileResult::Pass; // This should be an error in the future!
                    },
                };

                rbx_item.route = Some(vfs_item.route().to_vec());
                rbx_item.name = rbx_name;

                TransformFileResult::Value(Some(rbx_item))
            },
            &VfsItem::Dir { ref children, .. } => {
                let init_item = match children.get(JSON_MODEL_INIT) {
                    Some(v) => v,
                    None => return TransformFileResult::Pass,
                };

                let mut rbx_item = match self.transform_file(plugins, init_item) {
                    TransformFileResult::Value(Some(item)) => item,
                    TransformFileResult::Value(None) | TransformFileResult::Pass => {
                        eprintln!("Inconsistency detected in JsonModelPlugin!");
                        return TransformFileResult::Pass;
                    },
                };

                rbx_item.name.clear();
                rbx_item.name.push_str(vfs_item.name());
                rbx_item.route = Some(vfs_item.route().to_vec());

                for (child_name, child_item) in children {
                    if child_name == init_item.name() {
                        continue;
                    }

                    match plugins.transform_file(child_item) {
                        Some(child_rbx_item) => {
                            rbx_item.children.push(child_rbx_item);
                        },
                        _ => {},
                    }
                }

                TransformFileResult::Value(Some(rbx_item))
            },
        }
    }

    fn handle_file_change(&self, route: &Route) -> FileChangeResult {
        let leaf = match route.last() {
            Some(v) => v,
            None => return FileChangeResult::Pass,
        };

        let is_init = leaf == JSON_MODEL_INIT;

        if is_init {
            let mut changed = route.clone();
            changed.pop();

            FileChangeResult::MarkChanged(Some(vec![changed]))
        } else {
            FileChangeResult::Pass
        }
    }
}
