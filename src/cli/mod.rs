//! Defines Rojo's CLI through structopt types.

mod build;
mod doc;
mod fmt_project;
mod init;
mod plugin;
mod serve;
mod upload;

use std::{
    borrow::Cow,
    env,
    error::Error,
    fmt,
    net::IpAddr,
    path::{Path, PathBuf},
    str::FromStr,
};

use structopt::StructOpt;
use thiserror::Error;

pub use self::build::BuildCommand;
pub use self::doc::DocCommand;
pub use self::fmt_project::FmtProjectCommand;
pub use self::init::{InitCommand, InitKind};
pub use self::plugin::{PluginCommand, PluginSubcommand};
pub use self::serve::*;
pub use self::upload::*;

/// Command line options that Rojo accepts, defined using the structopt crate.
#[derive(Debug, StructOpt)]
#[structopt(name = "Rojo", about, author)]
pub struct Options {
    #[structopt(flatten)]
    pub global: GlobalOptions,

    /// Subcommand to run in this invocation.
    #[structopt(subcommand)]
    pub subcommand: Subcommand,
}

#[derive(Debug, StructOpt)]
pub struct GlobalOptions {
    /// Sets verbosity level. Can be specified multiple times.
    #[structopt(long("verbose"), short, global(true), parse(from_occurrences))]
    pub verbosity: u8,

    /// Set color behavior. Valid values are auto, always, and never.
    #[structopt(long("color"), global(true), default_value("auto"))]
    pub color: ColorChoice,
}

#[derive(Debug, Clone, Copy)]
pub enum ColorChoice {
    Auto,
    Always,
    Never,
}

impl FromStr for ColorChoice {
    type Err = ColorChoiceParseError;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        match source {
            "auto" => Ok(ColorChoice::Auto),
            "always" => Ok(ColorChoice::Always),
            "never" => Ok(ColorChoice::Never),
            _ => Err(ColorChoiceParseError {
                attempted: source.to_owned(),
            }),
        }
    }
}

impl From<ColorChoice> for termcolor::ColorChoice {
    fn from(value: ColorChoice) -> Self {
        match value {
            ColorChoice::Auto => termcolor::ColorChoice::Auto,
            ColorChoice::Always => termcolor::ColorChoice::Always,
            ColorChoice::Never => termcolor::ColorChoice::Never,
        }
    }
}

impl From<ColorChoice> for env_logger::WriteStyle {
    fn from(value: ColorChoice) -> Self {
        match value {
            ColorChoice::Auto => env_logger::WriteStyle::Auto,
            ColorChoice::Always => env_logger::WriteStyle::Always,
            ColorChoice::Never => env_logger::WriteStyle::Never,
        }
    }
}

#[derive(Debug, Error)]
#[error("Invalid color choice '{attempted}'. Valid values are: auto, always, never")]
pub struct ColorChoiceParseError {
    attempted: String,
}

#[derive(Debug, StructOpt)]
pub enum Subcommand {
    Init(InitCommand),
    Serve(ServeCommand),
    Build(BuildCommand),
    Upload(UploadCommand),
    FmtProject(FmtProjectCommand),
    Doc(DocCommand),
    Plugin(PluginCommand),
}

/// Expose a Rojo project to the Rojo Studio plugin.
#[derive(Debug, StructOpt)]
pub struct ServeCommand {
    /// Path to the project to serve. Defaults to the current directory.
    #[structopt(default_value = "")]
    pub project: PathBuf,

    /// The IP address to listen on. Defaults to `127.0.0.1`.
    #[structopt(long)]
    pub address: Option<IpAddr>,

    /// The port to listen on. Defaults to the project's preference, or `34872` if
    /// it has none.
    #[structopt(long)]
    pub port: Option<u16>,
}

impl ServeCommand {
    pub fn absolute_project(&self) -> Cow<'_, Path> {
        resolve_path(&self.project)
    }
}

/// Builds the project and uploads it to Roblox.
#[derive(Debug, StructOpt)]
pub struct UploadCommand {
    /// Path to the project to upload. Defaults to the current directory.
    #[structopt(default_value = "")]
    pub project: PathBuf,

    /// Authenication cookie to use. If not specified, Rojo will attempt to find one from the system automatically.
    #[structopt(long)]
    pub cookie: Option<String>,

    /// Asset ID to upload to.
    #[structopt(long = "asset_id")]
    pub asset_id: u64,
}

impl UploadCommand {
    pub fn absolute_project(&self) -> Cow<'_, Path> {
        resolve_path(&self.project)
    }
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

pub(super) fn resolve_path(path: &Path) -> Cow<'_, Path> {
    if path.is_absolute() {
        Cow::Borrowed(path)
    } else {
        Cow::Owned(env::current_dir().unwrap().join(path))
    }
}
