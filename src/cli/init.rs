use snafu::Snafu;

use crate::cli::InitCommand;

#[derive(Debug, Snafu)]
pub struct InitError(Error);

#[derive(Debug, Snafu)]
enum Error {}

pub fn init(options: InitCommand) -> Result<(), InitError> {
    Ok(init_inner(options)?)
}

fn init_inner(_options: InitCommand) -> Result<(), Error> {
    unimplemented!("init command");
}
