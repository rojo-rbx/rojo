//! Defines Rojo's CLI through structopt types.

use std::path::PathBuf;

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct Options {
    /// Sets verbosity level. Can be specified multiple times.
    #[structopt(long, short, parse(from_occurrences))]
    pub verbosity: u8,

    #[structopt(subcommand)]
    pub command: Subcommand,
}

#[derive(Debug, StructOpt)]
pub enum Subcommand {
    /// Creates a new Rojo project.
    Init(InitCommand),

    /// Serves the project's files for use with the Rojo Studio plugin.
    Serve(ServeCommand),

    /// Generates a model or place file from the project.
    Build(BuildCommand),

    /// Generates a place or model file out of the project and uploads it to Roblox.
    Upload(UploadCommand),
}

#[derive(Debug, StructOpt)]
pub struct InitCommand {
    path: Option<PathBuf>,
    // TODO: kind
}

#[derive(Debug, StructOpt)]
pub struct ServeCommand {
    /// Path to the project to serve. Defaults to the current directory.
    pub project: Option<PathBuf>,

    /// The port to listen on. Defaults to the project's preference, or 34872 if
    /// it has none.
    #[structopt(long)]
    pub port: Option<u16>,
}

#[derive(Debug, StructOpt)]
pub struct BuildCommand {
    /// Path to the project to serve. Defaults to the current directory.
    pub project: Option<PathBuf>,

    /// Where to output the result.
    #[structopt(long, short)]
    pub output: PathBuf,
}

#[derive(Debug, StructOpt)]
pub struct UploadCommand {
    /// Path to the project to upload. Defaults to the current directory.j
    pub project: Option<PathBuf>,

    // TODO: 'kind' as place or model
    /// Authenication cookie to use. If not specified, Rojo will attempt to find one from the system automatically.
    #[structopt(long)]
    pub cookie: Option<String>,

    /// Asset ID to upload to.
    #[structopt(long = "asset_id")]
    pub asset_id: u64,
}
