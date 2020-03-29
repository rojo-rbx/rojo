use memofs::Vfs;
use reqwest::header::{ACCEPT, CONTENT_TYPE, COOKIE, USER_AGENT};
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
    let root_id = inner_tree.get_root_id();
    let root_instance = inner_tree.get_instance(root_id).unwrap();

    let encode_ids = match root_instance.class_name.as_str() {
        "DataModel" => root_instance.get_children_ids().to_vec(),
        _ => vec![root_id],
    };

    let mut buffer = Vec::new();

    log::trace!("Encoding XML model");
    let config = rbx_xml::EncodeOptions::new()
        .property_behavior(rbx_xml::EncodePropertyBehavior::WriteUnknown);

    rbx_xml::to_writer(&mut buffer, tree.inner(), &encode_ids, config)?;

    let url = format!(
        "https://data.roblox.com/Data/Upload.ashx?assetid={}",
        options.asset_id
    );

    log::trace!("POSTing to {}", url);
    let client = reqwest::Client::new();
    let mut response = client
        .post(&url)
        .header(COOKIE, format!(".ROBLOSECURITY={}", &cookie))
        .header(USER_AGENT, "Roblox/WinInet")
        .header("Requester", "Client")
        .header(CONTENT_TYPE, "application/xml")
        .header(ACCEPT, "application/json")
        .body(buffer)
        .send()?;

    if !response.status().is_success() {
        return Err(Error::RobloxApi {
            body: response.text()?,
        }
        .into());
    }

    Ok(())
}
