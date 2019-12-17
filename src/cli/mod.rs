//! Defines Rojo's CLI through structopt types.

mod build;
mod init;
mod serve;
mod upload;

use std::{env, error::Error, fmt, path::PathBuf, str::FromStr};

use structopt::StructOpt;

pub use self::build::*;
pub use self::init::*;
pub use self::serve::*;
pub use self::upload::*;

/// Trick used with structopt to get the initial working directory of the
/// process and store it for use in default values.
fn working_dir() -> &'static str {
    lazy_static::lazy_static! {
        static ref INITIAL_WORKING_DIR: String = {
            env::current_dir().unwrap().display().to_string()
        };
    }

    &INITIAL_WORKING_DIR
}

/// Command line options that Rojo accepts, defined using the structopt crate.
#[derive(Debug, StructOpt)]
#[structopt(name = "Rojo", about, author)]
pub struct Options {
    /// Sets verbosity level. Can be specified multiple times.
    #[structopt(long = "verbose", short, parse(from_occurrences))]
    pub verbosity: u8,

    /// Subcommand to run in this invocation.
    #[structopt(subcommand)]
    pub subcommand: Subcommand,
}

/// All of Rojo's subcommands.
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

/// Initializes a new Rojo project.
#[derive(Debug, StructOpt)]
pub struct InitCommand {
    /// Path to the place to create the project. Defaults to the current directory.
    #[structopt(default_value = &working_dir())]
    pub path: PathBuf,

    /// The kind of project to create, 'place' or 'model'. Defaults to place.
    #[structopt(long, default_value = "place")]
    pub kind: InitKind,
}

/// The templates we support for initializing a Rojo project.
#[derive(Debug, Clone, Copy)]
pub enum InitKind {
    /// A place that matches what File -> New does in Roblox Studio.
    Place,

    /// An empty model, suitable for a library or plugin.
    Model,
}

impl FromStr for InitKind {
    type Err = InitKindParseError;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        match source {
            "place" => Ok(InitKind::Place),
            "model" => Ok(InitKind::Model),
            _ => Err(InitKindParseError {
                attempted: source.to_owned(),
            }),
        }
    }
}

/// Error type for failing to parse an `InitKind`.
#[derive(Debug)]
pub struct InitKindParseError {
    attempted: String,
}

impl Error for InitKindParseError {}

impl fmt::Display for InitKindParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "Invalid init kind '{}'. Valid kinds are: place, model",
            self.attempted
        )
    }
}

/// Expose a Rojo project through a web server that can communicate with the
/// Rojo Roblox Studio plugin, or be visited by the user in the browser.
#[derive(Debug, StructOpt)]
pub struct ServeCommand {
    /// Path to the project to serve. Defaults to the current directory.
    #[structopt(default_value = &working_dir())]
    pub project: PathBuf,

    /// The port to listen on. Defaults to the project's preference, or 34872 if
    /// it has none.
    #[structopt(long)]
    pub port: Option<u16>,
}

/// Build a Rojo project into a file.
#[derive(Debug, StructOpt)]
pub struct BuildCommand {
    /// Path to the project to serve. Defaults to the current directory.
    #[structopt(default_value = &working_dir())]
    pub project: PathBuf,

    /// Where to output the result.
    #[structopt(long, short)]
    pub output: PathBuf,
}

/// Build and upload a Rojo project to Roblox.com.
#[derive(Debug, StructOpt)]
pub struct UploadCommand {
    /// Path to the project to upload. Defaults to the current directory.
    #[structopt(default_value = &working_dir())]
    pub project: PathBuf,

    /// The kind of asset to generate, 'place', or 'model'. Defaults to place.
    #[structopt(long, default_value = "place")]
    pub kind: UploadKind,

    /// Authenication cookie to use. If not specified, Rojo will attempt to find one from the system automatically.
    #[structopt(long)]
    pub cookie: Option<String>,

    /// Asset ID to upload to.
    #[structopt(long = "asset_id")]
    pub asset_id: u64,
}

/// The kind of asset to upload to the website. Affects what endpoints Rojo uses
/// and changes how the asset is built.
#[derive(Debug, Clone, Copy)]
pub enum UploadKind {
    /// Upload to a place.
    Place,

    /// Upload to a model-like asset, like a Model, Plugin, or Package.
    Model,
}

impl FromStr for UploadKind {
    type Err = UploadKindParseError;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        match source {
            "place" => Ok(UploadKind::Place),
            "model" => Ok(UploadKind::Model),
            _ => Err(UploadKindParseError {
                attempted: source.to_owned(),
            }),
        }
    }
}

/// Error type for failing to parse an `UploadKind`.
#[derive(Debug)]
pub struct UploadKindParseError {
    attempted: String,
}

impl Error for UploadKindParseError {}

impl fmt::Display for UploadKindParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "Invalid upload kind '{}'. Valid kinds are: place, model",
            self.attempted
        )
    }
}
