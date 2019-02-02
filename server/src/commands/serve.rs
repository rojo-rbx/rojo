use std::{
    path::PathBuf,
    sync::Arc,
};

use log::info;
use failure::Fail;

use crate::{
    project::{Project, ProjectLoadFuzzyError},
    web::Server,
    imfs::FsError,
    live_session::LiveSession,
};

const DEFAULT_PORT: u16 = 34872;

#[derive(Debug)]
pub struct ServeOptions {
    pub fuzzy_project_path: PathBuf,
    pub port: Option<u16>,
}

#[derive(Debug, Fail)]
pub enum ServeError {
   #[fail(display = "Project load error: {}", _0)]
   ProjectLoadError(#[fail(cause)] ProjectLoadFuzzyError),

   #[fail(display = "{}", _0)]
   FsError(#[fail(cause)] FsError),
}

impl_from!(ServeError {
    ProjectLoadFuzzyError => ProjectLoadError,
    FsError => FsError,
});

pub fn serve(options: &ServeOptions) -> Result<(), ServeError> {
    info!("Looking for project at {}", options.fuzzy_project_path.display());

    let project = Arc::new(Project::load_fuzzy(&options.fuzzy_project_path)?);
    project.check_compatibility();

    info!("Found project at {}", project.file_location.display());
    info!("Using project {:#?}", project);

    let live_session = Arc::new(LiveSession::new(Arc::clone(&project))?);
    let server = Server::new(Arc::clone(&live_session));

    let port = options.port
        .or(project.serve_port)
        .unwrap_or(DEFAULT_PORT);

    println!("Rojo server listening on port {}", port);

    server.listen(port);

    Ok(())
}