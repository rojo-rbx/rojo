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
    LuaModule,
    LuaClient,
    LuaServer,
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
            "lua_module" => Ok(SnapshotMiddleware::LuaModule),
            "lua_server" => Ok(SnapshotMiddleware::LuaServer),
            "lua_client" => Ok(SnapshotMiddleware::LuaClient),
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
            return snapshot_lua_init(
                context,
                vfs,
                plugin_env,
                &init_path,
                &path.file_name().unwrap().to_string_lossy(),
                "ModuleScript",
            );
        }

        let init_path = path.join("init.server.lua");
        if vfs.metadata(&init_path).with_not_found()?.is_some() {
            return snapshot_lua_init(
                context,
                vfs,
                plugin_env,
                &init_path,
                &path.file_name().unwrap().to_string_lossy(),
                "Script",
            );
        }

        let init_path = path.join("init.client.lua");
        if vfs.metadata(&init_path).with_not_found()?.is_some() {
            return snapshot_lua_init(
                context,
                vfs,
                plugin_env,
                &init_path,
                &path.file_name().unwrap().to_string_lossy(),
                "LocalScript",
            );
        }

        snapshot_dir(context, vfs, plugin_env, path)
    } else {
        let mut middleware: (Option<SnapshotMiddleware>, Option<String>) =
            plugin_env.middleware(path.to_str().unwrap())?;

        if !matches!(middleware, (Some(_), _)) {
            middleware = if let Ok(name) = path.file_name_trim_end(".lua") {
                match name {
                    "init" | "init.client" | "init.server" => (None, None),
                    _ => {
                        if let Ok(name) = path.file_name_trim_end(".server.lua") {
                            (Some(SnapshotMiddleware::LuaServer), Some(name.to_owned()))
                        } else if let Ok(name) = path.file_name_trim_end(".client.lua") {
                            (Some(SnapshotMiddleware::LuaClient), Some(name.to_owned()))
                        } else {
                            (Some(SnapshotMiddleware::LuaModule), Some(name.to_owned()))
                        }
                    }
                }
            } else if path.file_name_ends_with(".project.json") {
                (
                    Some(SnapshotMiddleware::Project),
                    match path.file_name_trim_end(".project.json") {
                        Ok(v) => Some(v.to_owned()),
                        Err(_) => None,
                    },
                )
            } else if path.file_name_ends_with(".model.json") {
                (
                    Some(SnapshotMiddleware::JsonModel),
                    match path.file_name_trim_end(".model.json") {
                        Ok(v) => Some(v.to_owned()),
                        Err(_) => None,
                    },
                )
            } else if path.file_name_ends_with(".meta.json") {
                // .meta.json files do not turn into their own instances.
                (None, None)
            } else if path.file_name_ends_with(".json") {
                (
                    Some(SnapshotMiddleware::Json),
                    match path.file_name_trim_end(".json") {
                        Ok(v) => Some(v.to_owned()),
                        Err(_) => None,
                    },
                )
            } else if path.file_name_ends_with(".csv") {
                (
                    Some(SnapshotMiddleware::Csv),
                    match path.file_name_trim_end(".csv") {
                        Ok(v) => Some(v.to_owned()),
                        Err(_) => None,
                    },
                )
            } else if path.file_name_ends_with(".txt") {
                (
                    Some(SnapshotMiddleware::Txt),
                    match path.file_name_trim_end(".txt") {
                        Ok(v) => Some(v.to_owned()),
                        Err(_) => None,
                    },
                )
            } else if path.file_name_ends_with(".rbxmx") {
                (
                    Some(SnapshotMiddleware::Rbxmx),
                    match path.file_name_trim_end(".rbxmx") {
                        Ok(v) => Some(v.to_owned()),
                        Err(_) => None,
                    },
                )
            } else if path.file_name_ends_with(".rbxm") {
                (
                    Some(SnapshotMiddleware::Rbxm),
                    match path.file_name_trim_end(".rbxm") {
                        Ok(v) => Some(v.to_owned()),
                        Err(_) => None,
                    },
                )
            } else {
                (None, None)
            };
        }

        middleware = match middleware {
            // Pick a default name (name without extension)
            (Some(x), None) => (
                Some(x),
                match path.file_name_no_extension() {
                    Ok(v) => Some(v.to_owned()),
                    Err(_) => None,
                },
            ),
            x => x,
        };

        return match middleware {
            (Some(x), Some(name)) => match x {
                SnapshotMiddleware::LuaModule => {
                    snapshot_lua(context, vfs, &plugin_env, path, &name, "ModuleScript")
                }
                SnapshotMiddleware::LuaServer => {
                    snapshot_lua(context, vfs, &plugin_env, path, &name, "Script")
                }
                SnapshotMiddleware::LuaClient => {
                    snapshot_lua(context, vfs, &plugin_env, path, &name, "LocalScript")
                }
                SnapshotMiddleware::Project => snapshot_project(context, vfs, plugin_env, path),
                SnapshotMiddleware::JsonModel => {
                    snapshot_json_model(context, vfs, plugin_env, path, &name)
                }
                SnapshotMiddleware::Json => snapshot_json(context, vfs, plugin_env, path, &name),
                SnapshotMiddleware::Csv => snapshot_csv(context, vfs, plugin_env, path, &name),
                SnapshotMiddleware::Txt => snapshot_txt(context, vfs, plugin_env, path, &name),
                SnapshotMiddleware::Rbxmx => snapshot_rbxmx(context, vfs, plugin_env, path, &name),
                SnapshotMiddleware::Rbxm => snapshot_rbxm(context, vfs, plugin_env, path, &name),
                _ => Ok(None),
            },
            _ => Ok(None),
        };
    }
}
