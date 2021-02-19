use memofs::Vfs;
use reqwest::{
    header::{ACCEPT, CONTENT_TYPE, COOKIE, USER_AGENT},
    StatusCode,
};
use thiserror::Error;

use crate::{auth_cookie::get_auth_cookie, cli::UploadCommand, serve_session::ServeSession};

#[derive(Debug, Error)]
enum Error {
    #[error("Rojo could not find your Roblox auth cookie. Please pass one via --cookie.")]
    NeedAuthCookie,

    #[error("The Roblox API returned an unexpected error: {body}")]
    RobloxApi { body: String },
}

pub fn upload(options: UploadCommand) -> Result<(), anyhow::Error> {
    let cookie = options
        .cookie
        .clone()
        .or_else(get_auth_cookie)
        .ok_or(Error::NeedAuthCookie)?;

    let vfs = Vfs::new_default();

    let session = ServeSession::new(vfs, &options.absolute_project())?;

    let tree = session.tree();
    let inner_tree = tree.inner();
    let root = inner_tree.root();

    let encode_ids = match root.class.as_str() {
        "DataModel" => root.children().to_vec(),
        _ => vec![root.referent()],
    };

    let mut buffer = Vec::new();

    log::trace!("Encoding XML model");
    let config = rbx_xml::EncodeOptions::new()
        .property_behavior(rbx_xml::EncodePropertyBehavior::WriteUnknown);

    rbx_xml::to_writer(&mut buffer, tree.inner(), &encode_ids, config)?;
    do_upload(buffer, options.asset_id, &cookie)
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
        return Err(Error::RobloxApi {
            body: response.text()?,
        }
        .into());
    }

    Ok(())
}
