use std::{
    fs,
    io::{self, Write},
    net::{IpAddr, Ipv4Addr},
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{bail, Context};
use clap::Parser;
use memofs::Vfs;
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

use crate::{serve_session::ServeSession, web::LiveServer};

use super::{resolve_path, GlobalOptions};

const DEFAULT_BIND_ADDRESS: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);
const DEFAULT_PORT: u16 = 34872;

/// Expose a Rojo-compatible project to the Prism Studio plugin.
#[derive(Debug, Parser)]
pub struct ServeCommand {
    /// Path to the project to serve. If omitted, Prism discovers a project in
    /// the current directory.
    pub project: Option<PathBuf>,

    /// The IP address to listen on. Defaults to `127.0.0.1`.
    #[clap(long)]
    pub address: Option<IpAddr>,

    /// The port to listen on. Defaults to the project's preference, or `34872` if
    /// it has none.
    #[clap(long)]
    pub port: Option<u16>,

    /// Extra `Host`/`Origin` values the server will accept, beyond localhost and
    /// the bind address (for example a hostname like `mypc.lan`). Repeat the
    /// option or comma-separate to allow several. When given, this overrides the
    /// project's `serveAllowedHosts`. Listing any host also turns on Host/Origin
    /// validation for binds where it is otherwise off (such as `0.0.0.0`).
    #[clap(long, value_delimiter = ',')]
    pub allowed_hosts: Vec<String>,
}

impl ServeCommand {
    pub fn run(self, global: GlobalOptions) -> anyhow::Result<()> {
        let (project_path, auto_discovered) = resolve_project_path(self.project.as_deref())?;
        if auto_discovered {
            log::info!("Auto-discovered Prism project {}", project_path.display());
        }

        let vfs = Vfs::new_default()?;

        let session = Arc::new(ServeSession::new(vfs, project_path)?);

        let ip = self
            .address
            .or_else(|| session.serve_address())
            .unwrap_or(DEFAULT_BIND_ADDRESS.into());

        let port = self
            .port
            .or_else(|| session.project_port())
            .unwrap_or(DEFAULT_PORT);

        // The CLI flag, when given, replaces the project's list rather than
        // merging with it, matching how --address and --port override theirs.
        let allowed_hosts = if self.allowed_hosts.is_empty() {
            session.serve_allowed_hosts().to_vec()
        } else {
            self.allowed_hosts
        };

        let server = LiveServer::new(session);

        server.start((ip, port).into(), allowed_hosts, || {
            let _ = show_start_message(ip, port, global.color.into());
        })?;

        Ok(())
    }
}

fn show_start_message(bind_address: IpAddr, port: u16, color: ColorChoice) -> io::Result<()> {
    let mut green = ColorSpec::new();
    green.set_fg(Some(Color::Green)).set_bold(true);

    let writer = BufferWriter::stdout(color);
    let mut buffer = writer.buffer();

    let address_string = if bind_address.is_loopback() {
        "localhost".to_owned()
    } else {
        bind_address.to_string()
    };

    writeln!(&mut buffer, "Prism server listening:")?;

    write!(&mut buffer, "  Address: ")?;
    buffer.set_color(&green)?;
    writeln!(&mut buffer, "{}", address_string)?;

    buffer.set_color(&ColorSpec::new())?;
    write!(&mut buffer, "  Port:    ")?;
    buffer.set_color(&green)?;
    writeln!(&mut buffer, "{}", port)?;

    writeln!(&mut buffer)?;

    if !bind_address.is_loopback() {
        let mut warning = ColorSpec::new();
        warning.set_fg(Some(Color::Yellow)).set_bold(true);

        buffer.set_color(&warning)?;
        writeln!(
            &mut buffer,
            "WARNING: This server is bound to {address_string}, which is reachable from the \
             network.\n\
             The serve API is unauthenticated, so anyone who can reach {address_string}:{port} \
             can read\n\
             and modify your project's source. Prefer binding to localhost and tunneling (e.g. \
             SSH,\n\
             Tailscale, or WireGuard) when you need remote access."
        )?;
        buffer.set_color(&ColorSpec::new())?;
        writeln!(&mut buffer)?;
    }

    buffer.set_color(&ColorSpec::new())?;
    write!(&mut buffer, "Visit ")?;

    buffer.set_color(&green)?;
    write!(&mut buffer, "http://{}:{}/", address_string, port)?;

    buffer.set_color(&ColorSpec::new())?;
    writeln!(&mut buffer, " in your browser for more information.")?;

    writer.print(&buffer)?;

    Ok(())
}

fn resolve_project_path(explicit: Option<&Path>) -> anyhow::Result<(PathBuf, bool)> {
    if let Some(explicit) = explicit {
        return Ok((resolve_path(explicit)?.into_owned(), false));
    }

    let current_dir = std::env::current_dir().context(
        "Could not determine the current working directory for Prism project discovery.",
    )?;
    discover_project_in(&current_dir).map(|path| (path, true))
}

fn discover_project_in(current_dir: &Path) -> anyhow::Result<PathBuf> {
    let default_project = current_dir.join("default.project.json");
    if default_project.exists() {
        return Ok(default_project);
    }

    let entries = fs::read_dir(current_dir).with_context(|| {
        format!(
            "Could not inspect '{}' for Prism project files.",
            current_dir.display()
        )
    })?;
    let mut candidates = Vec::new();
    for entry in entries {
        let entry = entry.with_context(|| {
            format!(
                "Could not inspect an entry in '{}' during Prism project discovery.",
                current_dir.display()
            )
        })?;
        if entry
            .file_type()
            .with_context(|| format!("Could not inspect '{}'.", entry.path().display()))?
            .is_file()
            && entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.ends_with(".project.json"))
        {
            candidates.push(entry.path());
        }
    }
    candidates.sort();

    match candidates.as_slice() {
        [project] => Ok(project.clone()),
        [] => bail!(
            "Prism could not find a project in '{}'. Create default.project.json or pass an explicit project path.",
            current_dir.display()
        ),
        _ => {
            let candidates = candidates
                .iter()
                .map(|path| {
                    format!(
                        "  {}",
                        path.file_name()
                            .expect("project candidate has a file name")
                            .to_string_lossy()
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            bail!(
                "Prism found multiple project files in '{}':\n{}\nPass the project path explicitly.",
                current_dir.display(),
                candidates
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_project(path: &Path) {
        fs::write(path, r#"{"name":"Test","tree":{"$className":"DataModel"}}"#).unwrap();
    }

    #[test]
    fn explicit_path_is_used_without_discovery() {
        let directory = tempfile::tempdir().unwrap();
        write_project(&directory.path().join("other.project.json"));
        let explicit = directory.path().join("chosen.project.json");

        let (actual, auto_discovered) = resolve_project_path(Some(&explicit)).unwrap();

        assert_eq!(actual, explicit);
        assert!(!auto_discovered);
    }

    #[test]
    fn explicit_directory_is_used_unchanged() {
        let directory = tempfile::tempdir().unwrap();
        let (actual, auto_discovered) = resolve_project_path(Some(directory.path())).unwrap();

        assert_eq!(actual, directory.path());
        assert!(!auto_discovered);
    }

    #[test]
    fn default_project_has_priority() {
        let directory = tempfile::tempdir().unwrap();
        write_project(&directory.path().join("z.project.json"));
        write_project(&directory.path().join("default.project.json"));

        assert_eq!(
            discover_project_in(directory.path()).unwrap(),
            directory.path().join("default.project.json")
        );
    }

    #[test]
    fn exactly_one_alternate_project_is_discovered() {
        let directory = tempfile::tempdir().unwrap();
        write_project(&directory.path().join("game.project.json"));

        assert_eq!(
            discover_project_in(directory.path()).unwrap(),
            directory.path().join("game.project.json")
        );
    }

    #[test]
    fn no_project_has_a_helpful_error() {
        let directory = tempfile::tempdir().unwrap();
        let error = discover_project_in(directory.path())
            .unwrap_err()
            .to_string();

        assert!(error.contains("could not find a project"));
        assert!(error.contains("default.project.json"));
        assert!(error.contains("explicit project path"));
    }

    #[test]
    fn multiple_projects_are_sorted_and_require_an_explicit_path() {
        let directory = tempfile::tempdir().unwrap();
        for name in ["z.project.json", "a.project.json", "middle.project.json"] {
            write_project(&directory.path().join(name));
        }

        let error = discover_project_in(directory.path())
            .unwrap_err()
            .to_string();
        assert!(error.contains("multiple project files"));
        assert!(error.contains("Pass the project path explicitly"));
        assert!(error.find("a.project.json").unwrap() < error.find("middle.project.json").unwrap());
        assert!(error.find("middle.project.json").unwrap() < error.find("z.project.json").unwrap());
    }

    #[test]
    fn malformed_discovered_project_is_reported_by_session_loading() {
        let directory = tempfile::tempdir().unwrap();
        let project = directory.path().join("broken.project.json");
        fs::write(&project, "{not json").unwrap();
        let project = discover_project_in(directory.path()).unwrap();

        let error = match ServeSession::new(Vfs::new_default().unwrap(), project) {
            Ok(_) => panic!("malformed project unexpectedly loaded"),
            Err(error) => error.to_string(),
        };
        assert!(!error.is_empty());
    }

    #[test]
    fn unreadable_explicit_project_is_reported_by_session_loading() {
        let directory = tempfile::tempdir().unwrap();
        let missing = directory.path().join("missing.project.json");

        let error = match ServeSession::new(Vfs::new_default().unwrap(), missing) {
            Ok(_) => panic!("missing project unexpectedly loaded"),
            Err(error) => error.to_string(),
        };
        assert!(!error.is_empty());
    }
}
