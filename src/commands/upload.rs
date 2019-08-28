use std::path::PathBuf;

use failure::Fail;

use crate::auth_cookie::get_auth_cookie;

#[derive(Debug, Fail)]
pub enum UploadError {
    #[fail(display = "Rojo could not find your Roblox auth cookie. Please pass one via --cookie.")]
    NeedAuthCookie,
}

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

    unimplemented!("TODO: Reimplement upload command");
}
