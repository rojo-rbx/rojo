use std::path::PathBuf;

use failure::Fail;
use reqwest::header::{ACCEPT, CONTENT_TYPE, COOKIE, USER_AGENT};

use crate::{
    auth_cookie::get_auth_cookie,
    common_setup,
    vfs::{RealFetcher, Vfs, WatchMode},
};

#[derive(Debug, Fail)]
pub enum UploadError {
    #[fail(display = "Rojo could not find your Roblox auth cookie. Please pass one via --cookie.")]
    NeedAuthCookie,

    #[fail(display = "XML model file encode error: {}", _0)]
    XmlModelEncode(#[fail(cause)] rbx_xml::EncodeError),

    #[fail(display = "HTTP error: {}", _0)]
    Http(#[fail(cause)] reqwest::Error),

    #[fail(display = "Roblox API error: {}", _0)]
    RobloxApi(String),
}

impl_from!(UploadError {
    rbx_xml::EncodeError => XmlModelEncode,
    reqwest::Error => Http,
});

#[derive(Debug)]
pub struct UploadOptions<'a> {
    pub fuzzy_project_path: PathBuf,
    pub auth_cookie: Option<String>,
    pub asset_id: u64,
    pub kind: Option<&'a str>,
}

pub fn upload(options: UploadOptions) -> Result<(), UploadError> {
    let cookie = options
        .auth_cookie
        .or_else(get_auth_cookie)
        .ok_or(UploadError::NeedAuthCookie)?;

    log::trace!("Constructing in-memory filesystem");
    let mut vfs = Vfs::new(RealFetcher::new(WatchMode::Disabled));

    let (_maybe_project, tree) = common_setup::start(&options.fuzzy_project_path, &mut vfs);
    let root_id = tree.get_root_id();

    let mut buffer = Vec::new();

    log::trace!("Encoding XML model");
    let config = rbx_xml::EncodeOptions::new()
        .property_behavior(rbx_xml::EncodePropertyBehavior::WriteUnknown);
    rbx_xml::to_writer(&mut buffer, tree.inner(), &[root_id], config)?;

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
        return Err(UploadError::RobloxApi(response.text()?));
    }

    Ok(())
}
