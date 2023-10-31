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
mod toml;
mod txt;
mod util;

use std::path::{Path, PathBuf};

use anyhow::Context;
use memofs::{IoResultExt, Vfs};
use serde::{Deserialize, Serialize};

use crate::snapshot::{InstanceContext, InstanceSnapshot};

use self::{
    csv::{snapshot_csv, snapshot_csv_init},
    dir::snapshot_dir,
    json::snapshot_json,
    json_model::snapshot_json_model,
    lua::{snapshot_lua, snapshot_lua_init, ScriptType},
    project::snapshot_project,
    rbxm::snapshot_rbxm,
    rbxmx::snapshot_rbxmx,
    toml::snapshot_toml,
    txt::snapshot_txt,
    util::PathExt,
};

pub use self::{project::snapshot_project_node, util::emit_legacy_scripts_default};

/// Returns an `InstanceSnapshot` for the provided path.
/// This will inspect the path and find the appropriate middleware for it,
/// taking user-written rules into account. Then, it will attempt to convert
/// the path into an InstanceSnapshot using that middleware.
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
            match Middleware::from_path(context, &init_path) {
                Some(Middleware::Project) => snapshot_project(context, vfs, &init_path),

                Some(Middleware::ModuleScript) => {
                    snapshot_lua_init(context, vfs, &init_path, ScriptType::Module)
                }
                Some(Middleware::ServerScript) => {
                    snapshot_lua_init(context, vfs, &init_path, ScriptType::Server)
                }
                Some(Middleware::ClientScript) => {
                    snapshot_lua_init(context, vfs, &init_path, ScriptType::Client)
                }

                Some(Middleware::Csv) => snapshot_csv_init(context, vfs, &init_path),

                Some(_) | None => snapshot_dir(context, vfs, path),
            }
        } else {
            snapshot_dir(context, vfs, path)
        }
    } else {
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .with_context(|| format!("file name of {} is invalid", path.display()))?;

        match file_name {
            "init.server.luau" | "init.server.lua" | "init.client.luau" | "init.client.lua"
            | "init.luau" | "init.lua" | "init.csv" => return Ok(None),
            _ => {}
        }

        snapshot_from_path(context, vfs, path)
    }
}

/// Gets an `init` path for the given directory.
/// This uses an intrinsic priority list and for compatibility,
/// it should not be changed.
fn get_init_path<P: AsRef<Path>>(vfs: &Vfs, dir: P) -> anyhow::Result<Option<PathBuf>> {
    let path = dir.as_ref();

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

/// Gets a snapshot for a path given an InstanceContext and Vfs.
///
/// This is independent of the actual middleware and this function
/// should be used when possible. The middleware enum assumes that it is being
/// used as an override, and as a result Scripts do not have their paths
/// trimmed properly if it's used directly.
fn snapshot_from_path(
    context: &InstanceContext,
    vfs: &Vfs,
    path: &Path,
) -> anyhow::Result<Option<InstanceSnapshot>> {
    let (middleware, name) = if let Some(rule) = context.get_sync_rule(path) {
        (rule.middleware, rule.file_name_for_path(path)?)
    } else if path.file_name_ends_with(".server.lua") {
        (
            Middleware::ServerScript,
            path.file_name_trim_end(".server.lua")?,
        )
    } else if path.file_name_ends_with(".server.luau") {
        (
            Middleware::ServerScript,
            path.file_name_trim_end(".server.luau")?,
        )
    } else if path.file_name_ends_with(".client.lua") {
        (
            Middleware::ClientScript,
            path.file_name_trim_end(".client.lua")?,
        )
    } else if path.file_name_ends_with(".client.luau") {
        (
            Middleware::ClientScript,
            path.file_name_trim_end(".client.luau")?,
        )
    } else if path.file_name_ends_with(".lua") {
        (Middleware::ModuleScript, path.file_name_trim_end(".lua")?)
    } else if path.file_name_ends_with(".luau") {
        (Middleware::ModuleScript, path.file_name_trim_end(".luau")?)
    } else if path.file_name_ends_with(".project.json") {
        (
            Middleware::Project,
            path.file_name_trim_end(".project.json")?,
        )
    } else if path.file_name_ends_with(".model.json") {
        (
            Middleware::JsonModel,
            path.file_name_trim_end(".model.json")?,
        )
    } else if path.file_name_ends_with(".meta.json") {
        // .meta.json files do not turn into InstanceSnapshots
        return Ok(None);
    } else if path.file_name_ends_with(".json") {
        (Middleware::Json, path.file_name_trim_end(".json")?)
    } else if path.file_name_ends_with(".toml") {
        (Middleware::Toml, path.file_name_trim_end(".toml")?)
    } else if path.file_name_ends_with(".csv") {
        (Middleware::Csv, path.file_name_trim_end(".csv")?)
    } else if path.file_name_ends_with(".txt") {
        (Middleware::Text, path.file_name_trim_end(".txt")?)
    } else if path.file_name_ends_with(".rbxmx") {
        (Middleware::Rbxmx, path.file_name_trim_end(".rbxmx")?)
    } else if path.file_name_ends_with(".rbxm") {
        (Middleware::Rbxm, path.file_name_trim_end(".rbxm")?)
    } else {
        return Ok(None);
    };

    middleware.snapshot(context, vfs, path, name)
}

/// Represents a possible 'transformer' used by Rojo to turn a file system
/// item into a Roblox Instance. Missing from this list are directories and
/// metadata. This is deliberate, as metadata is not a snapshot middleware
/// and directories do not make sense to turn into files.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Middleware {
    Csv,
    JsonModel,
    Json,
    ServerScript,
    ClientScript,
    ModuleScript,
    Project,
    Rbxm,
    Rbxmx,
    Toml,
    Text,
    Ignore,
}

impl Middleware {
    /// Returns a `Middleware` from the provided path, taking user-specified
    /// syncing rules into account. If one exists, it is returned. Otherwise,
    /// `None` is returned.
    fn from_path<P: AsRef<Path>>(context: &InstanceContext, path: P) -> Option<Middleware> {
        let path = path.as_ref();

        if let Some(rule) = context.get_sync_rule(path) {
            Some(rule.middleware)
        } else if path.file_name_ends_with(".server.lua")
            || path.file_name_ends_with(".server.luau")
        {
            Some(Middleware::ServerScript)
        } else if path.file_name_ends_with(".client.lua")
            || path.file_name_ends_with(".client.luau")
        {
            Some(Middleware::ClientScript)
        } else if path.file_name_ends_with(".lua") || path.file_name_ends_with(".luau") {
            Some(Middleware::ModuleScript)
        } else if path.file_name_ends_with(".project.json") {
            Some(Middleware::Project)
        } else if path.file_name_ends_with(".model.json") {
            Some(Middleware::JsonModel)
        } else if path.file_name_ends_with(".meta.json") {
            // .meta.json files do not turn into their own instances.
            None
        } else if path.file_name_ends_with(".json") {
            Some(Middleware::Json)
        } else if path.file_name_ends_with(".toml") {
            Some(Middleware::Toml)
        } else if path.file_name_ends_with(".csv") {
            Some(Middleware::Csv)
        } else if path.file_name_ends_with(".txt") {
            Some(Middleware::Text)
        } else if path.file_name_ends_with(".rbxmx") {
            Some(Middleware::Rbxmx)
        } else if path.file_name_ends_with(".rbxm") {
            Some(Middleware::Rbxm)
        } else {
            None
        }
    }

    /// Creates a snapshot for the given path from the Middleware with
    /// the provided name.
    fn snapshot(
        &self,
        context: &InstanceContext,
        vfs: &Vfs,
        path: &Path,
        name: &str,
    ) -> anyhow::Result<Option<InstanceSnapshot>> {
        match self {
            Self::Csv => snapshot_csv(context, vfs, path, name),
            Self::JsonModel => snapshot_json_model(context, vfs, path, name),
            Self::Json => snapshot_json(context, vfs, path, name),
            Self::ServerScript => snapshot_lua(context, vfs, path, name, ScriptType::Server),
            Self::ClientScript => snapshot_lua(context, vfs, path, name, ScriptType::Client),
            Self::ModuleScript => snapshot_lua(context, vfs, path, name, ScriptType::Module),
            // At the moment, snapshot_project does not use `name` so we
            // don't provide it.
            Self::Project => snapshot_project(context, vfs, path),
            Self::Rbxm => snapshot_rbxm(context, vfs, path, name),
            Self::Rbxmx => snapshot_rbxmx(context, vfs, path, name),
            Self::Toml => snapshot_toml(context, vfs, path, name),
            Self::Text => snapshot_txt(context, vfs, path, name),
            Self::Ignore => Ok(None),
        }
    }
}
