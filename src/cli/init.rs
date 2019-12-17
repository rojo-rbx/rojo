use failure::Fail;

use crate::{cli::InitCommand, project::ProjectError};

#[derive(Debug, Fail)]
pub enum InitError {
    #[fail(display = "Project init error: {}", _0)]
    ProjectError(#[fail(cause)] ProjectError),
}

impl_from!(InitError {
    ProjectError => ProjectError,
});

pub fn init(_options: InitCommand) -> Result<(), InitError> {
    unimplemented!("init command");
}
