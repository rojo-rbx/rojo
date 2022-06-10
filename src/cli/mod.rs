//! Defines Rojo's CLI through clap types.

mod build;
mod doc;
mod fmt_project;
mod init;
mod plugin;
mod serve;
mod sourcemap;
mod upload;

use std::{borrow::Cow, env, path::Path, str::FromStr};

use clap::Parser;
use thiserror::Error;

pub use self::build::BuildCommand;
pub use self::doc::DocCommand;
pub use self::fmt_project::FmtProjectCommand;
pub use self::init::{InitCommand, InitKind};
pub use self::plugin::{PluginCommand, PluginSubcommand};
pub use self::serve::ServeCommand;
pub use self::sourcemap::SourcemapCommand;
pub use self::upload::UploadCommand;

/// Command line options that Rojo accepts, defined using the clap crate.
#[derive(Debug, Parser)]
#[clap(name = "Rojo", version, about, author)]
pub struct Options {
    #[clap(flatten)]
    pub global: GlobalOptions,

    /// Subcommand to run in this invocation.
    #[clap(subcommand)]
    pub subcommand: Subcommand,
}

impl Options {
    pub fn run(self) -> anyhow::Result<()> {
        match self.subcommand {
            Subcommand::Init(subcommand) => subcommand.run(),
            Subcommand::Serve(subcommand) => subcommand.run(self.global),
            Subcommand::Build(subcommand) => subcommand.run(),
            Subcommand::Upload(subcommand) => subcommand.run(),
            Subcommand::Sourcemap(subcommand) => subcommand.run(),
            Subcommand::FmtProject(subcommand) => subcommand.run(),
            Subcommand::Doc(subcommand) => subcommand.run(),
            Subcommand::Plugin(subcommand) => subcommand.run(),
        }
    }
}

#[derive(Debug, Parser)]
pub struct GlobalOptions {
    /// Sets verbosity level. Can be specified multiple times.
    #[clap(long("verbose"), short, global(true), parse(from_occurrences))]
    pub verbosity: u8,

    /// Set color behavior. Valid values are auto, always, and never.
    #[clap(long("color"), global(true), default_value("auto"))]
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

#[derive(Debug, Parser)]
pub enum Subcommand {
    Init(InitCommand),
    Serve(ServeCommand),
    Build(BuildCommand),
    Upload(UploadCommand),
    Sourcemap(SourcemapCommand),
    FmtProject(FmtProjectCommand),
    Doc(DocCommand),
    Plugin(PluginCommand),
}

pub(super) fn resolve_path(path: &Path) -> Cow<'_, Path> {
    if path.is_absolute() {
        Cow::Borrowed(path)
    } else {
        Cow::Owned(env::current_dir().unwrap().join(path))
    }
}
