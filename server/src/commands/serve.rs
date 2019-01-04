use std::{
    path::PathBuf,
    sync::Arc,
};

use failure::Fail;

use crate::{
    project::{Project, ProjectLoadFuzzyError},
    web::Server,
    session::Session,
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
}

impl From<ProjectLoadFuzzyError> for ServeError {
    fn from(error: ProjectLoadFuzzyError) -> ServeError {
        ServeError::ProjectLoadError(error)
    }
}

pub fn serve(options: &ServeOptions) -> Result<(), ServeError> {
    info!("Looking for project at {}", options.fuzzy_project_path.display());

    let project = Arc::new(Project::load_fuzzy(&options.fuzzy_project_path)?);

    info!("Found project at {}", project.file_location.display());
    info!("Using project {:#?}", project);

    let session = Arc::new(Session::new(Arc::clone(&project)).unwrap());
    let server = Server::new(Arc::clone(&session));

    let port = options.port
        .or(project.serve_port)
        .unwrap_or(DEFAULT_PORT);

    println!("Rojo server listening on port {}", port);

    server.listen(port);

    Ok(())
}