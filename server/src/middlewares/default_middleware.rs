use std::collections::HashMap;

use core::Route;
use middleware::{Middleware, MiddlewareChain, TransformFileResult, RbxChangeResult, FileChangeResult};
use rbx::{RbxInstance, RbxValue};
use vfs::VfsItem;

/// A middleware with simple transforms:
/// * Directories become Folder instances
/// * Files become StringValue objects with 'Value' as their contents
pub struct DefaultMiddleware;

impl DefaultMiddleware {
    pub fn new() -> DefaultMiddleware {
        DefaultMiddleware
    }
}

impl Middleware for DefaultMiddleware {
    fn transform_file(&self, middlewares: &MiddlewareChain, vfs_item: &VfsItem) -> TransformFileResult {
        match vfs_item {
            &VfsItem::File { ref contents, .. } => {
                let mut properties = HashMap::new();

                properties.insert("Value".to_string(), RbxValue::String {
                    value: contents.clone(),
                });

                TransformFileResult::Value(Some(RbxInstance {
                    name: vfs_item.name().clone(),
                    class_name: "StringValue".to_string(),
                    children: Vec::new(),
                    properties,
                    route: Some(vfs_item.route().to_vec()),
                }))
            },
            &VfsItem::Dir { ref children, .. } => {
                let mut rbx_children = Vec::new();

                for (_, child_item) in children {
                    match middlewares.transform_file(child_item) {
                        Some(rbx_item) => {
                            rbx_children.push(rbx_item);
                        },
                        _ => {},
                    }
                }

                TransformFileResult::Value(Some(RbxInstance {
                    name: vfs_item.name().clone(),
                    class_name: "*".to_string(),
                    children: rbx_children,
                    properties: HashMap::new(),
                    route: Some(vfs_item.route().to_vec()),
                }))
            },
        }
    }

    fn handle_file_change(&self, route: &Route) -> FileChangeResult {
        FileChangeResult::MarkChanged(Some(vec![route.clone()]))
    }

    fn handle_rbx_change(&self, _route: &Route, _rbx_item: &RbxInstance) -> RbxChangeResult {
        RbxChangeResult::Pass
    }
}
