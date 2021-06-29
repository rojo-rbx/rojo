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

    /// Asset ID to upload to.
    #[structopt(long = "asset_id")]
    pub asset_id: u64,
}

impl UploadCommand {
    pub fn run(self) -> Result<(), anyhow::Error> {
        let project_path = resolve_path(&self.project);

        let cookie = self.cookie.or_else(get_auth_cookie).context(
            "Rojo could not find your Roblox auth cookie. Please pass one via --cookie.",
        )?;

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
        do_upload(buffer, self.asset_id, &cookie)
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
