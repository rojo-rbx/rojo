//! Defines Rojo's CLI through structopt types.

mod build;
mod doc;
mod init;
mod plugin;
mod serve;
mod upload;

use std::{
    borrow::Cow,
    env,
    error::Error,
    fmt,
    path::{Path, PathBuf},
    str::FromStr,
};

use structopt::StructOpt;
use thiserror::Error;

pub use self::build::*;
pub use self::doc::*;
pub use self::init::*;
pub use self::plugin::*;
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
    /// Creates a new Rojo project.
    Init(InitCommand),

    /// Serves the project's files for use with the Rojo Studio plugin.
    Serve(ServeCommand),

    /// Generates a model or place file from the project.
    Build(BuildCommand),

    /// Generates a place or model file out of the project and uploads it to Roblox.
    Upload(UploadCommand),

    /// Open Rojo's documentation in your browser.
    Doc,

    /// Manages Rojo's Roblox Studio plugin.
    Plugin(PluginCommand),
}

/// Initializes a new Rojo project.
#[derive(Debug, StructOpt)]
pub struct InitCommand {
    /// Path to the place to create the project. Defaults to the current directory.
    #[structopt(default_value = "")]
    pub path: PathBuf,

    /// The kind of project to create, 'place' or 'model'. Defaults to place.
    #[structopt(long, default_value = "place")]
    pub kind: InitKind,
}

impl InitCommand {
    pub fn absolute_path(&self) -> Cow<'_, Path> {
        resolve_path(&self.path)
    }
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
    #[structopt(default_value = "")]
    pub project: PathBuf,

    /// The port to listen on. Defaults to the project's preference, or 34872 if
    /// it has none.
    #[structopt(long)]
    pub port: Option<u16>,
}

impl ServeCommand {
    pub fn absolute_project(&self) -> Cow<'_, Path> {
        resolve_path(&self.project)
    }
}

/// Build a Rojo project into a file.
#[derive(Debug, StructOpt)]
pub struct BuildCommand {
    /// Path to the project to serve. Defaults to the current directory.
    #[structopt(default_value = "")]
    pub project: PathBuf,

    /// Where to output the result.
    #[structopt(long, short)]
    pub output: PathBuf,

    /// Whether to automatically rebuild when any input files change.
    #[structopt(long)]
    pub watch: bool,
}

impl BuildCommand {
    pub fn absolute_project(&self) -> Cow<'_, Path> {
        resolve_path(&self.project)
    }
}

/// Build and upload a Rojo project to Roblox.com.
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

fn resolve_path(path: &Path) -> Cow<'_, Path> {
    if path.is_absolute() {
        Cow::Borrowed(path)
    } else {
        Cow::Owned(env::current_dir().unwrap().join(path))
    }
}

#[derive(Debug, StructOpt)]
pub enum PluginSubcommand {
    /// Install the plugin in Roblox Studio's plugins folder. If the plugin is
    /// already installed, installing it again will overwrite the current plugin
    /// file.
    Install,

    /// Removes the plugin if it is installed.
    Uninstall,
}

/// Install Rojo's plugin.
#[derive(Debug, StructOpt)]
pub struct PluginCommand {
    #[structopt(subcommand)]
    subcommand: PluginSubcommand,
}
