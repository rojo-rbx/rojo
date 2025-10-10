use std::path::PathBuf;

use anyhow::{bail, Context};
use clap::Parser;
use memofs::Vfs;
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
