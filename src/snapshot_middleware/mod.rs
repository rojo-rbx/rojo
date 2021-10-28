//! Defines the semantics that Rojo uses to turn entries on the filesystem into
//! Roblox instances using the instance snapshot subsystem.
//!
//! These modules define how files turn into instances.

#![allow(dead_code)]

mod csv;
mod dir;
mod json;
mod json_model;
mod lua;
mod meta_file;
mod project;
mod rbxm;
mod rbxmx;
mod txt;
mod util;

use std::{path::Path, str::FromStr};

use memofs::{IoResultExt, Vfs};

use crate::{
    plugin_env::PluginEnv,
    snapshot::{InstanceContext, InstanceSnapshot},
};

use self::{
    csv::snapshot_csv,
    dir::snapshot_dir,
    json::snapshot_json,
    json_model::snapshot_json_model,
    lua::{snapshot_lua, snapshot_lua_init},
    project::snapshot_project,
    rbxm::snapshot_rbxm,
    rbxmx::snapshot_rbxmx,
    txt::snapshot_txt,
    util::PathExt,
};

pub use self::project::snapshot_project_node;

#[derive(Debug)]
pub enum SnapshotMiddleware {
    Csv,
    Dir,
    Json,
    JsonModel,
    Lua,
    Project,
    Rbxm,
    Rbxmx,
    Txt,
}

impl FromStr for SnapshotMiddleware {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "csv" => Ok(SnapshotMiddleware::Csv),
            "dir" => Ok(SnapshotMiddleware::Dir),
            "json" => Ok(SnapshotMiddleware::Json),
            "json_model" => Ok(SnapshotMiddleware::JsonModel),
            "lua" => Ok(SnapshotMiddleware::Lua),
            "project" => Ok(SnapshotMiddleware::Project),
            "rbxm" => Ok(SnapshotMiddleware::Rbxm),
            "rbxmx" => Ok(SnapshotMiddleware::Rbxmx),
            "txt" => Ok(SnapshotMiddleware::Txt),
            _ => Err(format!("Unknown snapshot middleware: {}", s)),
        }
    }
}

/// The main entrypoint to the snapshot function. This function can be pointed
/// at any path and will return something if Rojo knows how to deal with it.
pub fn snapshot_from_vfs(
    context: &InstanceContext,
    vfs: &Vfs,
    plugin_env: &PluginEnv,
    path: &Path,
) -> anyhow::Result<Option<InstanceSnapshot>> {
    let meta = match vfs.metadata(path).with_not_found()? {
        Some(meta) => meta,
        None => return Ok(None),
    };

    if meta.is_dir() {
        let project_path = path.join("default.project.json");
        if vfs.metadata(&project_path).with_not_found()?.is_some() {
            return snapshot_project(context, vfs, plugin_env, &project_path);
        }

        let init_path = path.join("init.lua");
        if vfs.metadata(&init_path).with_not_found()?.is_some() {
            return snapshot_lua_init(context, vfs, plugin_env, &init_path);
        }

        let init_path = path.join("init.server.lua");
        if vfs.metadata(&init_path).with_not_found()?.is_some() {
            return snapshot_lua_init(context, vfs, plugin_env, &init_path);
        }

        let init_path = path.join("init.client.lua");
        if vfs.metadata(&init_path).with_not_found()?.is_some() {
            return snapshot_lua_init(context, vfs, plugin_env, &init_path);
        }

        snapshot_dir(context, vfs, plugin_env, path)
    } else {
        let mut middleware = plugin_env.middleware(path.to_str().unwrap())?;

        if middleware.is_none() {
            middleware = if let Ok(name) = path.file_name_trim_end(".lua") {
                match name {
                    "init" | "init.client" | "init.server" => None,
                    _ => Some(SnapshotMiddleware::Lua),
                }
            } else if path.file_name_ends_with(".project.json") {
                Some(SnapshotMiddleware::Project)
            } else if path.file_name_ends_with(".model.json") {
                Some(SnapshotMiddleware::JsonModel)
            } else if path.file_name_ends_with(".meta.json") {
                // .meta.json files do not turn into their own instances.
                None
            } else if path.file_name_ends_with(".json") {
                Some(SnapshotMiddleware::Json)
            } else if path.file_name_ends_with(".csv") {
                Some(SnapshotMiddleware::Csv)
            } else if path.file_name_ends_with(".txt") {
                Some(SnapshotMiddleware::Txt)
            } else if path.file_name_ends_with(".rbxmx") {
                Some(SnapshotMiddleware::Rbxmx)
            } else if path.file_name_ends_with(".rbxm") {
                Some(SnapshotMiddleware::Rbxm)
            } else {
                None
            };
        }

        return match middleware {
            Some(x) => match x {
                SnapshotMiddleware::Lua => snapshot_lua(context, vfs, path),
                SnapshotMiddleware::Project => snapshot_project(context, vfs, plugin_env, path),
                SnapshotMiddleware::JsonModel => {
                    snapshot_json_model(context, vfs, plugin_env, path)
                }
                SnapshotMiddleware::Json => snapshot_json(context, vfs, path),
                SnapshotMiddleware::Csv => snapshot_csv(context, vfs, path),
                SnapshotMiddleware::Txt => snapshot_txt(context, vfs, path),
                SnapshotMiddleware::Rbxmx => snapshot_rbxmx(context, vfs, path),
                SnapshotMiddleware::Rbxm => snapshot_rbxm(context, vfs, path),
                _ => Ok(None),
            },
            None => Ok(None),
        };
    }
}
