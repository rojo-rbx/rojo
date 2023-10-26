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

use crate::{
    glob::Glob,
    snapshot::{InstanceContext, InstanceSnapshot},
};

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
            match Middleware::from_path(context, &init_path) {
                Some(Middleware::Project) => snapshot_project(context, vfs, &init_path),

                Some(Middleware::ModuleScript) => snapshot_lua_init(context, vfs, &init_path),
                Some(Middleware::ServerScript) => snapshot_lua_init(context, vfs, &init_path),
                Some(Middleware::ClientScript) => snapshot_lua_init(context, vfs, &init_path),

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

pub fn snapshot_from_path<P: AsRef<Path>>(
    context: &InstanceContext,
    vfs: &Vfs,
    path: P,
) -> anyhow::Result<Option<InstanceSnapshot>> {
    let path = path.as_ref();

    if let Some(middleware) = context.get_sync_rule(path) {
        middleware.snapshot(context, vfs, path)
    } else if path.file_name_ends_with(".server.lua") || path.file_name_ends_with(".server.luau") {
        snapshot_lua(context, vfs, path, None)
    } else if path.file_name_ends_with(".client.lua") || path.file_name_ends_with(".client.luau") {
        snapshot_lua(context, vfs, path, None)
    } else if path.file_name_ends_with(".lua") || path.file_name_ends_with(".luau") {
        snapshot_lua(context, vfs, path, None)
    } else if let Some(middleware) = Middleware::from_path(context, path) {
        middleware.snapshot(context, vfs, path)
    } else {
        Ok(None)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct SyncRule {
    #[serde(rename = "pattern")]
    glob: Glob,
    #[serde(rename = "use")]
    middleware: Middleware,
}

impl SyncRule {
    pub fn matches(&self, path: &Path) -> bool {
        self.glob.is_match(path)
    }

    pub fn middleware(&self) -> Middleware {
        self.middleware
    }
}

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
    pub fn from_path<P: AsRef<Path>>(context: &InstanceContext, path: P) -> Option<Middleware> {
        let path = path.as_ref();

        if let Some(middleware) = context.get_sync_rule(path) {
            Some(middleware)
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

    pub fn snapshot(
        &self,
        context: &InstanceContext,
        vfs: &Vfs,
        path: &Path,
    ) -> anyhow::Result<Option<InstanceSnapshot>> {
        match self {
            Self::Csv => snapshot_csv(context, vfs, path),
            Self::JsonModel => snapshot_json_model(context, vfs, path),
            Self::Json => snapshot_json(context, vfs, path),
            Self::ServerScript => snapshot_lua(context, vfs, path, Some(ScriptType::Server)),
            Self::ClientScript => snapshot_lua(context, vfs, path, Some(ScriptType::Client)),
            Self::ModuleScript => snapshot_lua(context, vfs, path, Some(ScriptType::Module)),
            Self::Project => snapshot_project(context, vfs, path),
            Self::Rbxm => snapshot_rbxm(context, vfs, path),
            Self::Rbxmx => snapshot_rbxmx(context, vfs, path),
            Self::Toml => snapshot_toml(context, vfs, path),
            Self::Text => snapshot_txt(context, vfs, path),
            Self::Ignore => Ok(None),
        }
    }
}
