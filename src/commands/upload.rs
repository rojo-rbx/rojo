use std::{collections::HashMap, path::PathBuf};

use failure::Fail;
use rbx_dom_weak::RbxInstanceProperties;
use reqwest::header::{ACCEPT, CONTENT_TYPE, COOKIE, USER_AGENT};

use crate::{
    auth_cookie::get_auth_cookie,
    imfs::{Imfs, RealFetcher, WatchMode},
    snapshot::{apply_patch_set, compute_patch_set, InstancePropertiesWithMeta, RojoTree},
    snapshot_middleware::snapshot_from_imfs,
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

    let mut tree = RojoTree::new(InstancePropertiesWithMeta {
        properties: RbxInstanceProperties {
            name: "ROOT".to_owned(),
            class_name: "Folder".to_owned(),
            properties: HashMap::new(),
        },
        metadata: Default::default(),
    });
    let root_id = tree.get_root_id();

    log::trace!("Constructing in-memory filesystem");
    let mut imfs = Imfs::new(RealFetcher::new(WatchMode::Disabled));

    log::trace!("Reading project root");
    let entry = imfs
        .get(&options.fuzzy_project_path)
        .expect("could not get project path");

    log::trace!("Generating snapshot of instances from IMFS");
    let snapshot = snapshot_from_imfs(&mut imfs, &entry)
        .expect("snapshot failed")
        .expect("snapshot did not return an instance");

    log::trace!("Computing patch set");
    let patch_set = compute_patch_set(&snapshot, &tree, root_id);

    log::trace!("Applying patch set");
    apply_patch_set(&mut tree, patch_set);

    let root_id = tree.get_root_id();

    let mut buffer = Vec::new();

    let config = rbx_xml::EncodeOptions::new()
        .property_behavior(rbx_xml::EncodePropertyBehavior::WriteUnknown);
    rbx_xml::to_writer(&mut buffer, tree.inner(), &[root_id], config)?;

    let url = format!(
        "https://data.roblox.com/Data/Upload.ashx?assetid={}",
        options.asset_id
    );

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
