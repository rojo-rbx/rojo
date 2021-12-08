use std::path::PathBuf;
use std::str::FromStr;

use anyhow::{bail, format_err, Context};
use memofs::Vfs;
use reqwest::{
    header::{ACCEPT, CONTENT_TYPE, COOKIE, USER_AGENT},
    StatusCode,
};
use structopt::StructOpt;

use crate::{auth_cookie::get_auth_cookie, serve_session::ServeSession};

use super::resolve_path;

/// Builds the project and uploads it to Roblox.
#[derive(Debug, StructOpt)]
pub struct UploadCommand {
    /// Path to the project to upload. Defaults to the current directory.
    #[structopt(default_value = "")]
    pub project: PathBuf,

    /// Authenication cookie to use. If not specified, Rojo will attempt to find one from the system automatically.
    #[structopt(long)]
    pub cookie: Option<String>,

    /// Used for the new Open Cloud API. Still experimental.
    #[structopt(long = "api_key")]
    pub api_key: Option<String>,

    /// Required when using the Open Cloud API.
    #[structopt(long = "universe_id")]
    pub universe_id: Option<u64>,

    /// Asset ID to upload to.
    #[structopt(long = "asset_id")]
    pub asset_id: u64,
}

impl UploadCommand {
    pub fn run(self) -> Result<(), anyhow::Error> {
        let project_path = resolve_path(&self.project);

        let use_open_cloud = self.api_key.is_some();

        // Validate differently depending on if we're trying to use open cloud or not.
        // If there's a better way of doing this, please do so.
        let api_key = if use_open_cloud {
            self.api_key
                .context("Rojo could not find your api key. Please pass one via --api_key")?
        } else {
            "undefined".to_string()
        };
        let universe_id = if use_open_cloud {
            self.universe_id.context(
				"A Universe id is required when using the Open Cloud API. Please pass one via --universe_id"
			)?
        } else {
            0
        };
        let cookie = if use_open_cloud {
            "undefined".to_string()
        } else {
            self.cookie.or_else(get_auth_cookie).context(
                "Rojo could not find your Roblox auth cookie. Please pass one via --cookie.",
            )?
        };

        let vfs = Vfs::new_default();

        let session = ServeSession::new(vfs, project_path)?;

        let tree = session.tree();
        let inner_tree = tree.inner();
        let root = inner_tree.root();

        let encode_ids = match root.class.as_str() {
            "DataModel" => root.children().to_vec(),
            _ => vec![root.referent()],
        };

        let mut buffer = Vec::new();

        log::trace!("Encoding binary model");
        rbx_binary::to_writer(&mut buffer, tree.inner(), &encode_ids)?;
        if use_open_cloud {
            return do_upload_open_cloud(buffer, universe_id, self.asset_id, &api_key);
        } else {
            return do_upload(buffer, self.asset_id, &cookie);
        }
    }
}

/// The kind of asset to upload to the website. Affects what endpoints Rojo uses
/// and changes how the asset is built.
#[derive(Debug, Clone, Copy)]
enum UploadKind {
    /// Upload to a place.
    Place,

    /// Upload to a model-like asset, like a Model, Plugin, or Package.
    Model,
}

impl FromStr for UploadKind {
    type Err = anyhow::Error;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        match source {
            "place" => Ok(UploadKind::Place),
            "model" => Ok(UploadKind::Model),
            attempted => Err(format_err!(
                "Invalid upload kind '{}'. Valid kinds are: place, model",
                attempted
            )),
        }
    }
}

fn do_upload(buffer: Vec<u8>, asset_id: u64, cookie: &str) -> anyhow::Result<()> {
    let url = format!(
        "https://data.roblox.com/Data/Upload.ashx?assetid={}",
        asset_id
    );

    let client = reqwest::Client::new();

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
/// I'm sure there's a better of doing this. Please correct it if so.
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

    let client = reqwest::Client::new();

    let build_request = move || {
        client
            .post(&url)
            .header("x-api-key", api_key)
            .header(CONTENT_TYPE, "application/xml")
            .header(ACCEPT, "application/json")
            .body(buffer)
    };

    log::debug!("Uploading to Roblox...");
    let mut response = build_request().send()?;

    // Previously would've attempted to complete a CSRF challenge.
    // That should not be required using this API.(hopefully)

    let status = response.status();
    if !status.is_success() {
        bail!(
            "The Roblox API returned an unexpected error: {}",
            response.text()?
        );
    }

    Ok(())
}
