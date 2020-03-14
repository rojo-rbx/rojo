use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::Path,
};

use snafu::Snafu;

use crate::cli::{InitCommand, InitKind};

static DEFAULT_PLACE_PROJECT: &str = include_str!("../../assets/place.project.json");

static DEFAULT_MODEL_PROJECT: &str = include_str!("../../assets/model.project.json");
static DEFAULT_MODEL_INIT: &str = include_str!("../../assets/default-model.lua");

#[derive(Debug, Snafu)]
pub struct InitError(Error);

#[derive(Debug, Snafu)]
enum Error {
    #[snafu(display("A project file named default.project.json already exists in this folder"))]
    AlreadyExists,

    #[snafu(display("I/O error"))]
    Io { source: io::Error },
}

impl From<io::Error> for Error {
    fn from(source: io::Error) -> Self {
        Self::Io { source }
    }
}

pub fn init(options: InitCommand) -> Result<(), InitError> {
    Ok(init_inner(options)?)
}

fn init_inner(options: InitCommand) -> Result<(), Error> {
    let base_path = options.absolute_path();
    let canonical = fs::canonicalize(&base_path)?;
    let project_name = canonical
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("new-project");

    let project_params = ProjectParams {
        name: project_name.to_owned(),
    };

    match options.kind {
        InitKind::Place => init_place(&base_path, project_params),
        InitKind::Model => init_model(&base_path, project_params),
    }
}

fn init_place(base_path: &Path, project_params: ProjectParams) -> Result<(), Error> {
    let project_file = project_params.render_template(DEFAULT_PLACE_PROJECT);
    try_create_project(base_path, &project_file)?;

    let src = base_path.join("src");
    fs::create_dir_all(&src)?;

    let src_common = src.join("common");
    fs::create_dir_all(src.join(&src_common))?;

    let src_server = src.join("server");
    fs::create_dir_all(src.join(&src_server))?;

    let src_client = src.join("client");
    fs::create_dir_all(src.join(&src_client))?;

    write_if_not_exists(
        &src_common.join("Hello.lua"),
        "return function()\n\tprint(\"Hello, world!\")\nend",
    )?;

    write_if_not_exists(
        &src_server.join("init.server.lua"),
        "print(\"Hello world, from server!\")",
    )?;

    write_if_not_exists(
        &src_client.join("init.client.lua"),
        "print(\"Hello world, from client!\")",
    )?;

    Ok(())
}

fn init_model(base_path: &Path, project_params: ProjectParams) -> Result<(), Error> {
    let project_file = project_params.render_template(DEFAULT_MODEL_PROJECT);
    try_create_project(base_path, &project_file)?;

    let src = base_path.join("src");
    fs::create_dir_all(&src)?;

    let init = project_params.render_template(DEFAULT_MODEL_INIT);
    write_if_not_exists(&src.join("init.lua"), &init)?;

    Ok(())
}

/// Contains parameters used in templates to create a project.
struct ProjectParams {
    name: String,
}

impl ProjectParams {
    /// Render a template by replacing variables with project parameters.
    fn render_template(&self, template: &str) -> String {
        template.replace("{project_name}", &self.name)
    }
}

/// Write a file if it does not exist yet, otherwise, leave it alone.
fn write_if_not_exists(path: &Path, contents: &str) -> Result<(), Error> {
    let file_res = OpenOptions::new().write(true).create_new(true).open(path);

    let mut file = match file_res {
        Ok(file) => file,
        Err(err) => {
            return match err.kind() {
                io::ErrorKind::AlreadyExists => return Ok(()),
                _ => Err(err.into()),
            }
        }
    };

    file.write_all(contents.as_bytes())?;

    Ok(())
}

/// Try to create a project file and fail if it already exists.
fn try_create_project(base_path: &Path, contents: &str) -> Result<(), Error> {
    let project_path = base_path.join("default.project.json");

    let file_res = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(project_path);

    let mut file = match file_res {
        Ok(file) => file,
        Err(err) => {
            return match err.kind() {
                io::ErrorKind::AlreadyExists => Err(Error::AlreadyExists),
                _ => Err(err.into()),
            }
        }
    };

    file.write_all(contents.as_bytes())?;

    Ok(())
}
