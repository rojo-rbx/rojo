use std::path::PathBuf;

use failure::Fail;

#[derive(Debug, Fail)]
pub enum InitError {
    #[fail(
        display = "Invalid project kind '{}', valid kinds are 'place' and 'model'",
        _0
    )]
    InvalidKind(String),
}

#[derive(Debug)]
pub struct InitOptions<'a> {
    pub fuzzy_project_path: PathBuf,
    pub kind: Option<&'a str>,
}

pub fn init(_options: &InitOptions) -> Result<(), InitError> {
    unimplemented!("init command");
}
