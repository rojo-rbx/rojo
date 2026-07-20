//! Defines Prism's CLI through clap types.

mod automation;
mod build;
mod doc;
mod exec;
mod fmt_project;
mod init;
mod inspect;
mod plugin;
mod serve;
mod sourcemap;
mod syncback;
mod upload;

use std::{borrow::Cow, env, panic, path::Path, process, str::FromStr};

use anyhow::Context;
use backtrace::Backtrace;
#[cfg(test)]
use clap::CommandFactory;
use clap::Parser;
use thiserror::Error;

pub use self::build::BuildCommand;
pub use self::doc::DocCommand;
pub use self::exec::ExecCommand;
pub use self::fmt_project::FmtProjectCommand;
pub use self::init::{InitCommand, InitKind};
pub use self::inspect::InspectCommand;
pub use self::plugin::{PluginCommand, PluginSubcommand};
pub use self::serve::ServeCommand;
pub use self::sourcemap::SourcemapCommand;
pub use self::syncback::SyncbackCommand;
pub use self::upload::UploadCommand;

/// Command line options that Prism accepts, defined using the clap crate.
#[derive(Debug, Parser)]
#[clap(
    name = "Prism",
    version,
    about = "Prism developer tooling for Roblox, derived from Rojo"
)]
pub struct Options {
    #[clap(flatten)]
    pub global: GlobalOptions,

    /// Subcommand to run in this invocation.
    #[clap(subcommand)]
    pub subcommand: Subcommand,
}

pub fn run() {
    #[cfg(feature = "profile-with-tracy")]
    profiling::tracy_client::Client::start();

    panic::set_hook(Box::new(|panic_info| {
        let message = match panic_info.payload().downcast_ref::<&str>() {
            Some(&message) => message.to_string(),
            None => match panic_info.payload().downcast_ref::<String>() {
                Some(message) => message.clone(),
                None => "<no message>".to_string(),
            },
        };

        log::error!(
            "Prism crashed! You are running Prism {}.",
            env!("CARGO_PKG_VERSION")
        );
        log::error!("This is probably a Prism bug.");
        log::error!("");
        log::error!("Please report this Prism build through its distribution channel.");
        log::error!("");
        log::error!("Details: {}", message);

        if let Some(location) = panic_info.location() {
            log::error!("in file {} on line {}", location.file(), location.line());
        }

        let should_backtrace = env::var("RUST_BACKTRACE")
            .map(|var| var == "1")
            .unwrap_or(false);

        if should_backtrace {
            eprintln!("{:?}", Backtrace::new());
        } else {
            eprintln!(
                "note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace."
            );
        }

        process::exit(1);
    }));

    let options = Options::parse();

    let log_filter = match options.global.verbosity {
        0 => "info",
        1 => "info,librojo=debug",
        2 => "info,librojo=trace",
        _ => "trace",
    };

    let log_env = env_logger::Env::default().default_filter_or(log_filter);

    env_logger::Builder::from_env(log_env)
        .format_module_path(false)
        .format_timestamp(None)
        .format_indent(Some(8))
        .write_style(options.global.color.into())
        .init();

    if let Err(err) = options.run() {
        log::error!("{:?}", err);
        process::exit(1);
    }
}

impl Options {
    pub fn run(self) -> anyhow::Result<()> {
        match self.subcommand {
            Subcommand::Init(subcommand) => subcommand.run(),
            Subcommand::Serve(subcommand) => subcommand.run(self.global),
            Subcommand::Build(subcommand) => subcommand.run(),
            Subcommand::Exec(subcommand) => subcommand.run(),
            Subcommand::Inspect(subcommand) => subcommand.run(),
            Subcommand::Upload(subcommand) => subcommand.run(),
            Subcommand::Sourcemap(subcommand) => subcommand.run(),
            Subcommand::FmtProject(subcommand) => subcommand.run(),
            Subcommand::Doc(subcommand) => subcommand.run(),
            Subcommand::Plugin(subcommand) => subcommand.run(),
            Subcommand::Syncback(subcommand) => subcommand.run(self.global),
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
    Exec(ExecCommand),
    Inspect(InspectCommand),
    Upload(UploadCommand),
    Sourcemap(SourcemapCommand),
    FmtProject(FmtProjectCommand),
    Doc(DocCommand),
    Plugin(PluginCommand),
    Syncback(SyncbackCommand),
}

pub(super) fn resolve_path(path: &Path) -> anyhow::Result<Cow<'_, Path>> {
    if path.is_absolute() {
        Ok(Cow::Borrowed(path))
    } else {
        let current_dir = env::current_dir().context(
            "Could not determine the current working directory. \
             It may have been deleted, or Prism may not have permission to access it.",
        )?;
        Ok(Cow::Owned(current_dir.join(path)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_and_version_are_prism_branded() {
        let command = Options::command();
        assert_eq!(command.get_name(), "Prism");
        assert_eq!(command.get_version(), Some(env!("CARGO_PKG_VERSION")));

        let mut help = Vec::new();
        Options::command().write_long_help(&mut help).unwrap();
        let help = String::from_utf8(help).unwrap();
        assert!(help.contains("Prism developer tooling for Roblox"));
        assert!(help.contains("USAGE:\n    Prism"));
    }
}
