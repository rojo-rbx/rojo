use std::{
    io::BufReader,
    mem::forget,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use clap::Parser;
use fs_err::File;
use memofs::Vfs;
use rbx_dom_weak::{types::Ref, Instance, WeakDom};
use reqwest::{
    header::{ACCEPT, CONTENT_TYPE, COOKIE, USER_AGENT},
    StatusCode,
};

use crate::{auth_cookie::get_auth_cookie, serve_session::ServeSession};

use super::resolve_path;

/// Builds the project and uploads it to Roblox.
#[derive(Debug, Parser)]
pub struct UploadCommand {
    /// Path to the project to upload. Defaults to the current directory.
    #[clap(default_value = "")]
    pub project: PathBuf,

    /// Path to a base .rbxl or .rbxlx file to merge with the project before
    /// uploading. When provided, the Rojo project tree is merged into this
    /// file so that the upload contains both the base content (3D assets,
    /// terrain, lighting, etc.) and the project's scripts.
    #[clap(long)]
    pub base: Option<PathBuf>,

    /// Authenication cookie to use. If not specified, Rojo will attempt to find one from the system automatically.
    #[clap(long)]
    pub cookie: Option<String>,

    /// API key obtained from create.roblox.com/credentials. Rojo will use the Open Cloud API when this is provided. Only supports uploading to a place.
    #[clap(long = "api_key")]
    pub api_key: Option<String>,

    /// The Universe ID of the given place. Required when using the Open Cloud API.
    #[clap(long = "universe_id")]
    pub universe_id: Option<u64>,

    /// Asset ID to upload to.
    #[clap(long = "asset_id")]
    pub asset_id: u64,
}

impl UploadCommand {
    pub fn run(self) -> Result<(), anyhow::Error> {
        let project_path = resolve_path(&self.project);

        let vfs = Vfs::new_default();

        let session = ServeSession::new(vfs, project_path)?;

        let mut buffer = Vec::new();

        if let Some(base_path) = &self.base {
            let base_path = resolve_path(base_path);

            log::trace!("Reading base file: {}", base_path.display());
            let base_dom = read_base_file(&base_path)?;

            log::trace!("Merging Rojo project into base file");
            let merged_dom = merge_rojo_into_base(base_dom, session.tree().inner())?;

            let encode_ids = merged_dom.root().children().to_vec();

            log::trace!("Encoding merged binary model");
            rbx_binary::to_writer(&mut buffer, &merged_dom, &encode_ids)?;
        } else {
            let tree = session.tree();
            let inner_tree = tree.inner();
            let root = inner_tree.root();

            let encode_ids = match root.class.as_str() {
                "DataModel" => root.children().to_vec(),
                _ => vec![root.referent()],
            };

            log::trace!("Encoding binary model");
            rbx_binary::to_writer(&mut buffer, inner_tree, &encode_ids)?;
        }

        // Avoid dropping ServeSession: it's potentially very expensive to
        // drop and we're about to exit anyway.
        forget(session);

        match (self.cookie, self.api_key, self.universe_id) {
            (cookie, None, universe) => {
                // using legacy. notify if universe is provided.
                if universe.is_some() {
                    log::warn!(
                        "--universe_id was provided but is ignored when using legacy upload"
                    );
                }

                let cookie = cookie.or_else(get_auth_cookie).context(
                    "Rojo could not find your Roblox auth cookie. Please pass one via --cookie.",
                )?;
                do_upload(buffer, self.asset_id, &cookie)
            }

            (cookie, Some(api_key), Some(universe_id)) => {
                // using open cloud. notify if cookie is provided.
                if cookie.is_some() {
                    log::warn!("--cookie was provided but is ignored when using Open Cloud API");
                }

                do_upload_open_cloud(buffer, universe_id, self.asset_id, &api_key)
            }

            (_, Some(_), None) => {
                // API key is provided, universe id is not.
                bail!("--universe_id must be provided to use the Open Cloud API");
            }
        }
    }
}

/// Reads a base .rbxl or .rbxlx file into a WeakDom.
fn read_base_file(path: &Path) -> anyhow::Result<WeakDom> {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .context("Base file must have a .rbxl or .rbxlx extension")?;

    let content = BufReader::new(
        File::open(path)
            .with_context(|| format!("Could not open base file at {}", path.display()))?,
    );

    match extension {
        "rbxl" => rbx_binary::from_reader(content).with_context(|| {
            format!(
                "Could not deserialize binary place file at {}",
                path.display()
            )
        }),
        "rbxlx" => {
            let config = rbx_xml::DecodeOptions::new()
                .property_behavior(rbx_xml::DecodePropertyBehavior::ReadUnknown);
            rbx_xml::from_reader(content, config).with_context(|| {
                format!(
                    "Could not deserialize XML place file at {}",
                    path.display()
                )
            })
        }
        _ => bail!("Base file must be .rbxl or .rbxlx, got .{}", extension),
    }
}

/// Merges the Rojo project tree into a base WeakDom.
///
/// For each service that exists in the Rojo tree, finds the matching service
/// in the base DOM by name and className and recursively merges Rojo children
/// into it. Instances in the base that Rojo doesn't manage are left untouched,
/// preserving 3D assets, terrain, lighting, and other non-scripted content.
fn merge_rojo_into_base(mut base: WeakDom, rojo: &WeakDom) -> anyhow::Result<WeakDom> {
    let rojo_root = rojo.root();
    let base_root_ref = base.root_ref();

    if rojo_root.class.as_str() != "DataModel" {
        // Non-place project (model): clone the entire Rojo tree as a child
        let cloned_ref = rojo.clone_into_external(rojo_root.referent(), &mut base);
        base.transfer_within(cloned_ref, base_root_ref);
        return Ok(base);
    }

    merge_instances(&mut base, base_root_ref, rojo, rojo.root_ref());

    Ok(base)
}

/// Recursively merges the children of a Rojo instance into the corresponding
/// base instance.
///
/// For each Rojo child:
///   - If a matching child (by name+class) exists in base: update its
///     properties and recurse into its children
///   - If no match: clone the entire Rojo subtree into base under the
///     current parent
///
/// Base children that have no Rojo counterpart are never touched, preserving
/// 3D assets and other content that Rojo doesn't manage.
fn merge_instances(
    base: &mut WeakDom,
    base_parent_ref: Ref,
    rojo: &WeakDom,
    rojo_parent_ref: Ref,
) {
    for &rojo_child_ref in rojo.get_by_ref(rojo_parent_ref).unwrap().children() {
        let rojo_child = rojo.get_by_ref(rojo_child_ref).unwrap();

        let base_match = find_child_by_name_and_class(
            base,
            base_parent_ref,
            &rojo_child.name,
            rojo_child.class.as_str(),
        );

        match base_match {
            Some(base_child_ref) => {
                update_properties(base, base_child_ref, rojo_child);
                merge_instances(base, base_child_ref, rojo, rojo_child_ref);
            }
            None => {
                let cloned_ref = rojo.clone_into_external(rojo_child_ref, base);
                base.transfer_within(cloned_ref, base_parent_ref);
            }
        }
    }
}

/// Finds a child of `parent_ref` in the given DOM that matches both name and
/// className.
fn find_child_by_name_and_class(
    dom: &WeakDom,
    parent_ref: Ref,
    name: &str,
    class: &str,
) -> Option<Ref> {
    let parent = dom.get_by_ref(parent_ref)?;
    for &child_ref in parent.children() {
        let child = dom.get_by_ref(child_ref)?;
        if child.name == name && child.class.as_str() == class {
            return Some(child_ref);
        }
    }
    None
}

/// Overlays Rojo properties onto a base instance. Properties that exist only
/// in the base instance are preserved; properties from Rojo overwrite any
/// existing base values.
fn update_properties(base: &mut WeakDom, base_ref: Ref, rojo_instance: &Instance) {
    let base_instance = base.get_by_ref_mut(base_ref).unwrap();

    base_instance.name.clone_from(&rojo_instance.name);

    for (prop_name, prop_value) in &rojo_instance.properties {
        base_instance
            .properties
            .insert(*prop_name, prop_value.clone());
    }
}

fn do_upload(buffer: Vec<u8>, asset_id: u64, cookie: &str) -> anyhow::Result<()> {
    let url = format!(
        "https://data.roblox.com/Data/Upload.ashx?assetid={}",
        asset_id
    );

    let client = reqwest::blocking::Client::new();

    let build_request = move || {
        client
            .post(&url)
            .header(COOKIE, format!(".ROBLOSECURITY={}", cookie))
            .header(USER_AGENT, "Roblox/WinInet")
            .header(CONTENT_TYPE, "application/xml")
            .header(ACCEPT, "application/json")
            .body(buffer.clone())
    };

    log::debug!("Uploading to Roblox...");
    let mut response = build_request().send()?;

    // Starting in Feburary, 2021, the upload endpoint performs CSRF challenges.
    // If we receive an HTTP 403 with a X-CSRF-Token reply, we should retry the
    // request, echoing the value of that header.
    if response.status() == StatusCode::FORBIDDEN {
        if let Some(csrf_token) = response.headers().get("X-CSRF-Token") {
            log::debug!("Received CSRF challenge, retrying with token...");
            response = build_request().header("X-CSRF-Token", csrf_token).send()?;
        }
    }

    let status = response.status();
    if !status.is_success() {
        bail!(
            "The Roblox API returned an unexpected error: {}",
            response.text()?
        );
    }

    Ok(())
}

/// Implementation of do_upload that supports the new open cloud api.
/// see https://developer.roblox.com/en-us/articles/open-cloud
fn do_upload_open_cloud(
    buffer: Vec<u8>,
    universe_id: u64,
    asset_id: u64,
    api_key: &str,
) -> anyhow::Result<()> {
    let url = format!(
        "https://apis.roblox.com/universes/v1/{}/places/{}/versions?versionType=Published",
        universe_id, asset_id
    );

    let client = reqwest::blocking::Client::new();

    log::debug!("Uploading to Roblox...");
    let response = client
        .post(url)
        .header("x-api-key", api_key)
        .header(CONTENT_TYPE, "application/xml")
        .header(ACCEPT, "application/json")
        .body(buffer)
        .send()?;

    let status = response.status();
    if !status.is_success() {
        bail!(
            "The Roblox API returned an unexpected error: {}",
            response.text()?
        );
    }

    Ok(())
}
