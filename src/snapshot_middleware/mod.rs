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
mod yaml;

use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

use anyhow::Context;
use memofs::{IoResultExt, Vfs};
use serde::{Deserialize, Serialize};

use crate::glob::Glob;
use crate::snapshot::{InstanceContext, InstanceSnapshot, SyncRule};

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
    yaml::snapshot_yaml,
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
            // TODO: support user-defined init paths
            // If and when we do, make sure to go support it in
            // `Project::set_file_name`, as right now it special-cases
            // `default.project.json` as an `init` path.
            for rule in default_sync_rules() {
                if rule.matches(&init_path) {
                    return match rule.middleware {
                        Middleware::Project => {
                            let name = init_path
                                .parent()
                                .and_then(Path::file_name)
                                .and_then(|s| s.to_str()).expect("default.project.json should be inside a folder with a unicode name");
                            snapshot_project(context, vfs, &init_path, name)
                        }

                        Middleware::ModuleScript => {
                            snapshot_lua_init(context, vfs, &init_path, ScriptType::Module)
                        }
                        Middleware::ServerScript => {
                            snapshot_lua_init(context, vfs, &init_path, ScriptType::Server)
                        }
                        Middleware::ClientScript => {
                            snapshot_lua_init(context, vfs, &init_path, ScriptType::Client)
                        }

                        Middleware::Csv => snapshot_csv_init(context, vfs, &init_path),

                        _ => snapshot_dir(context, vfs, path),
                    };
                }
            }
            snapshot_dir(context, vfs, path)
        } else {
            snapshot_dir(context, vfs, path)
        }
    } else {
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .with_context(|| format!("file name of {} is invalid", path.display()))?;

        // TODO: Is this even necessary anymore?
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

/// Gets a snapshot for a path given an InstanceContext and Vfs, taking
/// user specified sync rules into account.
fn snapshot_from_path(
    context: &InstanceContext,
    vfs: &Vfs,
    path: &Path,
) -> anyhow::Result<Option<InstanceSnapshot>> {
    if let Some(rule) = context.get_user_sync_rule(path) {
        return rule
            .middleware
            .snapshot(context, vfs, path, rule.file_name_for_path(path)?);
    } else {
        for rule in default_sync_rules() {
            if rule.matches(path) {
                return rule.middleware.snapshot(
                    context,
                    vfs,
                    path,
                    rule.file_name_for_path(path)?,
                );
            }
        }
    }
    Ok(None)
}

/// Represents a possible 'transformer' used by Rojo to turn a file system
/// item into a Roblox Instance. Missing from this list are directories and
/// metadata. This is deliberate, as metadata is not a snapshot middleware
/// and directories do not make sense to turn into files.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Middleware {
    Csv,
    JsonModel,
    Json,
    ServerScript,
    ClientScript,
    ModuleScript,
    PluginScript,
    LegacyClientScript,
    LegacyServerScript,
    RunContextServerScript,
    RunContextClientScript,
    Project,
    Rbxm,
    Rbxmx,
    Toml,
    Text,
    Yaml,
    Ignore,
}

impl Middleware {
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
            Self::PluginScript => snapshot_lua(context, vfs, path, name, ScriptType::Plugin),
            Self::LegacyClientScript => {
                snapshot_lua(context, vfs, path, name, ScriptType::LegacyClient)
            }
            Self::LegacyServerScript => {
                snapshot_lua(context, vfs, path, name, ScriptType::LegacyServer)
            }
            Self::RunContextClientScript => {
                snapshot_lua(context, vfs, path, name, ScriptType::RunContextClient)
            }
            Self::RunContextServerScript => {
                snapshot_lua(context, vfs, path, name, ScriptType::RunContextServer)
            }
            Self::Project => snapshot_project(context, vfs, path, name),
            Self::Rbxm => snapshot_rbxm(context, vfs, path, name),
            Self::Rbxmx => snapshot_rbxmx(context, vfs, path, name),
            Self::Toml => snapshot_toml(context, vfs, path, name),
            Self::Text => snapshot_txt(context, vfs, path, name),
            Self::Yaml => snapshot_yaml(context, vfs, path, name),
            Self::Ignore => Ok(None),
        }
    }
}

/// A helper for easily defining a SyncRule. Arguments are passed literally
/// to this macro in the order `include`, `middleware`, `suffix`,
/// and `exclude`. Both `suffix` and `exclude` are optional.
///
/// All arguments except `middleware` are expected to be strings.
/// The `middleware` parameter is expected to be a variant of `Middleware`,
/// not including the enum name itself.
macro_rules! sync_rule {
    ($pattern:expr, $middleware:ident) => {
        SyncRule {
            middleware: Middleware::$middleware,
            include: Glob::new($pattern).unwrap(),
            exclude: None,
            suffix: None,
            base_path: PathBuf::new(),
        }
    };
    ($pattern:expr, $middleware:ident, $suffix:expr) => {
        SyncRule {
            middleware: Middleware::$middleware,
            include: Glob::new($pattern).unwrap(),
            exclude: None,
            suffix: Some($suffix.into()),
            base_path: PathBuf::new(),
        }
    };
    ($pattern:expr, $middleware:ident, $suffix:expr, $exclude:expr) => {
        SyncRule {
            middleware: Middleware::$middleware,
            include: Glob::new($pattern).unwrap(),
            exclude: Some(Glob::new($exclude).unwrap()),
            suffix: Some($suffix.into()),
            base_path: PathBuf::new(),
        }
    };
}

/// Defines the 'default' syncing rules that Rojo uses.
/// These do not broadly overlap, but the order matters for some in the case of
/// e.g. JSON models.
pub fn default_sync_rules() -> &'static [SyncRule] {
    static DEFAULT_SYNC_RULES: OnceLock<Vec<SyncRule>> = OnceLock::new();

    DEFAULT_SYNC_RULES.get_or_init(|| {
        vec![
            sync_rule!("*.server.lua", ServerScript, ".server.lua"),
            sync_rule!("*.server.luau", ServerScript, ".server.luau"),
            sync_rule!("*.client.lua", ClientScript, ".client.lua"),
            sync_rule!("*.client.luau", ClientScript, ".client.luau"),
            sync_rule!("*.plugin.lua", PluginScript, ".plugin.lua"),
            sync_rule!("*.plugin.luau", PluginScript, ".plugin.luau"),
            sync_rule!("*.{lua,luau}", ModuleScript),
            sync_rule!("*.project.json", Project, ".project.json"),
            sync_rule!("*.model.json", JsonModel, ".model.json"),
            sync_rule!("*.json", Json, ".json", "*.meta.json"),
            sync_rule!("*.toml", Toml),
            sync_rule!("*.csv", Csv),
            sync_rule!("*.txt", Text),
            sync_rule!("*.rbxmx", Rbxmx),
            sync_rule!("*.rbxm", Rbxm),
            sync_rule!("*.{yml,yaml}", Yaml),
        ]
    })
}
