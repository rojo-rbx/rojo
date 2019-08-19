use std::{
    path::PathBuf,
};

use failure::Fail;

const DEFAULT_PORT: u16 = 34872;

#[derive(Debug)]
pub struct ServeOptions {
    pub fuzzy_project_path: PathBuf,
    pub port: Option<u16>,
}

#[derive(Debug, Fail)]
pub enum ServeError {
    #[fail(display = "This error cannot happen.")]
    CannotHappen,
}

pub fn serve(options: &ServeOptions) -> Result<(), ServeError> {
    // TODO: Pull port from project iff it exists.

    let port = options.port
        // .or(project.serve_port)
        .unwrap_or(DEFAULT_PORT);

    println!("Rojo server listening on port {}", port);

    Ok(())
}