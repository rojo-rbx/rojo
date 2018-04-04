use regex::Regex;
use serde_json;

use core::Route;
use middleware::{Middleware, MiddlewareChain, TransformFileResult, RbxChangeResult, FileChangeResult};
use rbx::RbxInstance;
use vfs::VfsItem;

lazy_static! {
    static ref JSON_MODEL_PATTERN: Regex = Regex::new(r"^(.*?)\.model\.json$").unwrap();
}

pub struct JsonModelMiddleware;

impl JsonModelMiddleware {
    pub fn new() -> JsonModelMiddleware {
        JsonModelMiddleware
    }
}

impl Middleware for JsonModelMiddleware {
    fn transform_file(&self, _middlewares: &MiddlewareChain, vfs_item: &VfsItem) -> TransformFileResult {
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
            &VfsItem::Dir { .. } => TransformFileResult::Pass,
        }
    }

    fn handle_file_change(&self, _route: &Route) -> FileChangeResult {
        FileChangeResult::Pass
    }

    fn handle_rbx_change(&self, _route: &Route, _rbx_item: &RbxInstance) -> RbxChangeResult {
        RbxChangeResult::Pass
    }
}
