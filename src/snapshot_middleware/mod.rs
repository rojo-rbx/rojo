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

use std::path::{Path, PathBuf};

use anyhow::Context;
use memofs::{IoResultExt, Vfs};

use crate::snapshot::{InstanceContext, InstanceSnapshot, Transformer};

use self::{
    csv::{snapshot_csv, snapshot_csv_init},
    dir::snapshot_dir,
    json::snapshot_json,
    json_model::snapshot_json_model,
    lua::{snapshot_lua, snapshot_lua_init, ScriptType},
    project::snapshot_project,
    rbxm::snapshot_rbxm,
    rbxmx::snapshot_rbxmx,
    txt::snapshot_txt,
    util::PathExt,
};

pub use self::project::snapshot_project_node;

/// Returns the path of the first relevant `init` file in the given directory.
fn get_init_path(vfs: &Vfs, path: &Path) -> anyhow::Result<Option<PathBuf>> {
    let project_path = path.join("default.project.json");
    if vfs.metadata(&project_path).with_not_found()?.is_some() {
        return Ok(Some(project_path));
    }

    let init_path = path.join("init.luau");
    if vfs.metadata(&init_path).with_not_found()?.is_some() {
        return Ok(Some(init_path));
    }

    let init_path = path.join("init.lua");
    if vfs.metadata(&init_path).with_not_found()?.is_some() {
        return Ok(Some(init_path));
    }

    let init_path = path.join("init.server.luau");
    if vfs.metadata(&init_path).with_not_found()?.is_some() {
        return Ok(Some(init_path));
    }

    let init_path = path.join("init.server.lua");
    if vfs.metadata(&init_path).with_not_found()?.is_some() {
        return Ok(Some(init_path));
    }

    let init_path = path.join("init.client.luau");
    if vfs.metadata(&init_path).with_not_found()?.is_some() {
        return Ok(Some(init_path));
    }

    let init_path = path.join("init.client.lua");
    if vfs.metadata(&init_path).with_not_found()?.is_some() {
        return Ok(Some(init_path));
    }

    let init_path = path.join("init.csv");
    if vfs.metadata(&init_path).with_not_found()?.is_some() {
        return Ok(Some(init_path));
    }

    Ok(None)
}

/// Returns the rojo type for the object. Any override rules in the `context`
/// take precedence.
fn get_transformer(context: &InstanceContext, path: &Path) -> Option<Transformer> {
    if let Some(rojo_type) = context.get_transformer_override(path) {
        return Some(rojo_type);
    }

    if path.file_name_ends_with(".server.lua") || path.file_name_ends_with(".server.luau") {
        Some(Transformer::LuauServer)
    } else if path.file_name_ends_with(".client.lua") || path.file_name_ends_with(".client.luau") {
        Some(Transformer::LuauClient)
    } else if path.file_name_ends_with(".lua") || path.file_name_ends_with(".luau") {
        Some(Transformer::LuauModule)
    } else if path.file_name_ends_with(".project.json") {
        Some(Transformer::Project)
    } else if path.file_name_ends_with(".model.json") {
        Some(Transformer::JsonModel)
    } else if path.file_name_ends_with(".meta.json") {
        // .meta.json files do not turn into their own instances.
        None
    } else if path.file_name_ends_with(".json") {
        Some(Transformer::Json)
    } else if path.file_name_ends_with(".csv") {
        Some(Transformer::Csv)
    } else if path.file_name_ends_with(".txt") {
        Some(Transformer::Plain)
    } else if path.file_name_ends_with(".rbxmx") {
        Some(Transformer::Rbxmx)
    } else if path.file_name_ends_with(".rbxm") {
        Some(Transformer::Rbxm)
    } else {
        None
    }
}

/// The main entrypoint to the snapshot function. This function can be pointed
/// at any path and will return something if Rojo knows how to deal with it.
#[profiling::function]
pub fn snapshot_from_vfs(
    context: &InstanceContext,
    vfs: &Vfs,
    path: &Path,
) -> anyhow::Result<Option<InstanceSnapshot>> {
    let meta = match vfs.metadata(path).with_not_found()? {
        Some(meta) => meta,
        None => return Ok(None),
    };

    if meta.is_dir() {
        if let Some(init_path) = get_init_path(vfs, path)? {
            match get_transformer(context, &init_path) {
                Some(Transformer::Project) => return snapshot_project(context, vfs, &init_path),
                Some(Transformer::LuauModule) => {
                    return snapshot_lua_init(context, vfs, &init_path, Some(ScriptType::Module))
                }
                Some(Transformer::LuauServer) => {
                    return snapshot_lua_init(context, vfs, &init_path, Some(ScriptType::Server))
                }
                Some(Transformer::LuauClient) => {
                    return snapshot_lua_init(context, vfs, &init_path, Some(ScriptType::Client))
                }
                Some(Transformer::Csv) => return snapshot_csv_init(context, vfs, &init_path),

                Some(Transformer::Other(rojo_type_string)) => {
                    anyhow::bail!("Unknown rojo type: {}", rojo_type_string)
                }

                Some(_) | None => (),
            }
        }

        snapshot_dir(context, vfs, path)
    } else {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .with_context(|| format!("Path had an invalid file name: {}", path.display()))?;

        // Ignore files processed by the is_dir check above
        match file_name {
            "init.lua" | "init.luau" | "init.client.lua" | "init.client.luau"
            | "init.server.lua" | "init.server.luau" | "init.csv" => return Ok(None),
            _ => (),
        }

        match get_transformer(context, path) {
            Some(Transformer::Project) => snapshot_project(context, vfs, path),
            Some(Transformer::JsonModel) => snapshot_json_model(context, vfs, path),
            Some(Transformer::Json) => snapshot_json(context, vfs, path),
            Some(Transformer::Csv) => snapshot_csv(context, vfs, path),
            Some(Transformer::Plain) => snapshot_txt(context, vfs, path),
            Some(Transformer::LuauModule) => {
                snapshot_lua(context, vfs, path, Some(ScriptType::Module))
            }
            Some(Transformer::LuauServer) => {
                snapshot_lua(context, vfs, path, Some(ScriptType::Server))
            }
            Some(Transformer::LuauClient) => {
                snapshot_lua(context, vfs, path, Some(ScriptType::Client))
            }
            Some(Transformer::Rbxmx) => snapshot_rbxmx(context, vfs, path),
            Some(Transformer::Rbxm) => snapshot_rbxm(context, vfs, path),
            Some(Transformer::Other(rojo_type_string)) => {
                anyhow::bail!("Unknown rojo type: {}", rojo_type_string)
            }
            None | Some(Transformer::Ignore) => Ok(None),
        }
    }
}
