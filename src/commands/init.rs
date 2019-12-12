use failure::Fail;

use crate::{
    cli::{InitCommand, InitKind},
    project::{Project, ProjectInitError},
};

#[derive(Debug, Fail)]
pub enum InitError {
    #[fail(display = "Project init error: {}", _0)]
    ProjectInitError(#[fail(cause)] ProjectInitError),
}

impl_from!(InitError {
    ProjectInitError => ProjectInitError,
});

pub fn init(options: InitCommand) -> Result<(), InitError> {
    let (project_path, project_kind) = match options.kind {
        InitKind::Place => {
            let path = Project::init_place(&options.path)?;
            (path, "place")
        }
        InitKind::Model => {
            let path = Project::init_model(&options.path)?;
            (path, "model")
        }
    };

    println!(
        "Created new {} project file at {}",
        project_kind,
        project_path.display()
    );

    Ok(())
}
