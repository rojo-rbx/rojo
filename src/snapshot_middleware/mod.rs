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

use std::path::Path;

use memofs::{IoResultExt, Vfs};

use crate::snapshot::{InstanceContext, InstanceSnapshot};

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
    util::match_file_name,
};

pub use self::project::snapshot_project_node;

/// The main entrypoint to the snapshot function. This function can be pointed
/// at any path and will return something if Rojo knows how to deal with it.
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
        let project_path = path.join("default.project.json");
        if vfs.metadata(&project_path).with_not_found()?.is_some() {
            return snapshot_project(context, vfs, &project_path);
        }

        let init_path = path.join("init.lua");
        if vfs.metadata(&init_path).with_not_found()?.is_some() {
            return snapshot_lua_init(context, vfs, &init_path);
        }

        let init_path = path.join("init.server.lua");
        if vfs.metadata(&init_path).with_not_found()?.is_some() {
            return snapshot_lua_init(context, vfs, &init_path);
        }

        let init_path = path.join("init.client.lua");
        if vfs.metadata(&init_path).with_not_found()?.is_some() {
            return snapshot_lua_init(context, vfs, &init_path);
        }

        snapshot_dir(context, vfs, path)
    } else {
        if let Some(name) = match_file_name(path, ".lua") {
            match name {
                // init scripts are handled elsewhere and should not turn into
                // their own children.
                "init" | "init.client" | "init.server" => return Ok(None),

                _ => return snapshot_lua(context, vfs, path),
            }
        } else if let Some(_name) = match_file_name(path, ".project.json") {
            return snapshot_project(context, vfs, path);
        } else if let Some(name) = match_file_name(path, ".model.json") {
            return snapshot_json_model(context, vfs, path, name);
        } else if let Some(_name) = match_file_name(path, ".meta.json") {
            // .meta.json files do not turn into their own instances.
            return Ok(None);
        } else if let Some(name) = match_file_name(path, ".json") {
            return snapshot_json(context, vfs, path, name);
        } else if let Some(name) = match_file_name(path, ".csv") {
            return snapshot_csv(context, vfs, path, name);
        } else if let Some(name) = match_file_name(path, ".txt") {
            return snapshot_txt(context, vfs, path, name);
        } else if let Some(name) = match_file_name(path, ".rbxmx") {
            return snapshot_rbxmx(context, vfs, path, name);
        } else if let Some(name) = match_file_name(path, ".rbxm") {
            return snapshot_rbxm(context, vfs, path, name);
        }

        Ok(None)
    }
}
