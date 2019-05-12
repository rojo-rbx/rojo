use std::{
    path::PathBuf,
    io,
};

use log::info;
use failure::Fail;

use reqwest::header::{ACCEPT, USER_AGENT, CONTENT_TYPE, COOKIE};

use crate::{
    imfs::{Imfs, FsError},
    project::{Project, ProjectLoadFuzzyError},
    rbx_session::construct_oneoff_tree,
    rbx_snapshot::SnapshotError,
};

#[derive(Debug, Fail)]
pub enum UploadError {
    #[fail(display = "Roblox API Error: {}", _0)]
    RobloxApiError(String),

    #[fail(display = "Invalid asset kind: {}", _0)]
    InvalidKind(String),

    #[fail(display = "Project load error: {}", _0)]
    ProjectLoadError(#[fail(cause)] ProjectLoadFuzzyError),

    #[fail(display = "IO error: {}", _0)]
    IoError(#[fail(cause)] io::Error),

    #[fail(display = "HTTP error: {}", _0)]
    HttpError(#[fail(cause)] reqwest::Error),

    #[fail(display = "XML model file error")]
    XmlModelEncodeError(rbx_xml::EncodeError),

    #[fail(display = "{}", _0)]
    FsError(#[fail(cause)] FsError),

    #[fail(display = "{}", _0)]
    SnapshotError(#[fail(cause)] SnapshotError),
}

impl_from!(UploadError {
    ProjectLoadFuzzyError => ProjectLoadError,
    io::Error => IoError,
    reqwest::Error => HttpError,
    rbx_xml::EncodeError => XmlModelEncodeError,
    FsError => FsError,
    SnapshotError => SnapshotError,
});

#[derive(Debug)]
pub struct UploadOptions<'a> {
    pub fuzzy_project_path: PathBuf,
    pub security_cookie: String,
    pub asset_id: u64,
    pub kind: Option<&'a str>,
}

pub fn upload(options: &UploadOptions) -> Result<(), UploadError> {
    // TODO: Switch to uploading binary format?

    info!("Looking for project at {}", options.fuzzy_project_path.display());

    let project = Project::load_fuzzy(&options.fuzzy_project_path)?;
    project.check_compatibility();

    info!("Found project at {}", project.file_location.display());
    info!("Using project {:#?}", project);

    let mut imfs = Imfs::new();
    imfs.add_roots_from_project(&project)?;
    let tree = construct_oneoff_tree(&project, &imfs)?;

    let root_id = tree.get_root_id();
    let mut contents = Vec::new();

    match options.kind {
        Some("place") | None => {
            let top_level_ids = tree.get_instance(root_id).unwrap().get_children_ids();
            rbx_xml::to_writer_default(&mut contents, &tree, top_level_ids)?;
        },
        Some("model") => {
            rbx_xml::to_writer_default(&mut contents, &tree, &[root_id])?;
        },
        Some(invalid) => return Err(UploadError::InvalidKind(invalid.to_owned())),
    }

    let url = format!("https://data.roblox.com/Data/Upload.ashx?assetid={}", options.asset_id);

    let client = reqwest::Client::new();
    let mut response = client.post(&url)
        .header(COOKIE, format!(".ROBLOSECURITY={}", &options.security_cookie))
        .header(USER_AGENT, "Roblox/WinInet")
        .header("Requester", "Client")
        .header(CONTENT_TYPE, "application/xml")
        .header(ACCEPT, "application/json")
        .body(contents)
        .send()?;

    if !response.status().is_success() {
        return Err(UploadError::RobloxApiError(response.text()?));
    }

    Ok(())
}