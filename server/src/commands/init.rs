use std::{
    path::PathBuf,
};

use failure::Fail;

use crate::project::{Project, ProjectInitError};

#[derive(Debug, Fail)]
pub enum InitError {
    #[fail(display = "Invalid project kind '{}', valid kinds are 'place' and 'model'", _0)]
    InvalidKind(String),

    #[fail(display = "Project init error: {}", _0)]
    ProjectInitError(#[fail(cause)] ProjectInitError)
}

impl From<ProjectInitError> for InitError {
    fn from(error: ProjectInitError) -> InitError {
        InitError::ProjectInitError(error)
    }
}

#[derive(Debug)]
pub struct InitOptions<'a> {
    pub fuzzy_project_path: PathBuf,
    pub kind: Option<&'a str>,
}

pub fn init(options: &InitOptions) -> Result<(), InitError> {
    let (project_path, project_kind) = match options.kind {
        Some("place") | None => {
            let path = Project::init_place(&options.fuzzy_project_path)?;
            (path, "place")
        },
        Some("model") => {
            let path = Project::init_model(&options.fuzzy_project_path)?;
            (path, "model")
        },
        Some(invalid) => return Err(InitError::InvalidKind(invalid.to_string())),
    };

    println!("Created new {} project at {}", project_kind, project_path.display());

    Ok(())
}