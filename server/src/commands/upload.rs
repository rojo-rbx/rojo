use std::path::PathBuf;

use failure::Fail;

#[derive(Debug, Fail)]
pub enum UploadError {
    #[fail(display = "This error cannot happen")]
    StubError,
}

#[derive(Debug)]
pub struct UploadOptions<'a> {
    pub fuzzy_project_path: PathBuf,
    pub security_cookie: String,
    pub asset_id: u64,
    pub kind: Option<&'a str>,
}

pub fn upload(_options: &UploadOptions) -> Result<(), UploadError> {
    unimplemented!("TODO: Reimplement upload command");
}