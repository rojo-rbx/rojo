use std::{
    io::{self, Write},
    net::{IpAddr, Ipv4Addr},
    path::PathBuf,
    sync::Arc,
};

use clap::Parser;
use memofs::Vfs;
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

use crate::{serve_session::ServeSession, web::LiveServer};

use super::{resolve_path, GlobalOptions};

const DEFAULT_BIND_ADDRESS: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);
const DEFAULT_PORT: u16 = 34872;

/// Expose a Rojo project to the Rojo Studio plugin.
#[derive(Debug, Parser)]
pub struct ServeCommand {
    /// Path to the project to serve. Defaults to the current directory.
    #[clap(default_value = "")]
    pub project: PathBuf,

    /// The IP address to listen on. Defaults to `127.0.0.1`.
    #[clap(long)]
    pub address: Option<IpAddr>,

    /// The port to listen on. Defaults to the project's preference, or `34872` if
    /// it has none.
    #[clap(long)]
    pub port: Option<u16>,
}

impl ServeCommand {
    pub fn run(self, global: GlobalOptions) -> anyhow::Result<()> {
        let project_path = resolve_path(&self.project);

        let vfs = Vfs::new_default();

        let session = Arc::new(ServeSession::new(vfs, &project_path)?);

        let ip = self
            .address
            .or_else(|| session.serve_address())
            .unwrap_or(DEFAULT_BIND_ADDRESS.into());

        let port = self
            .port
            .or_else(|| session.project_port())
            .unwrap_or(DEFAULT_PORT);

        let server = LiveServer::new(session);

        let _ = show_start_message(ip, port, global.color.into());
        server.start((ip, port).into());

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

    writeln!(&mut buffer, "Rojo server listening:")?;

    write!(&mut buffer, "  Address: ")?;
    buffer.set_color(&green)?;
    writeln!(&mut buffer, "{}", address_string)?;

    buffer.set_color(&ColorSpec::new())?;
    write!(&mut buffer, "  Port:    ")?;
    buffer.set_color(&green)?;
    writeln!(&mut buffer, "{}", port)?;

    writeln!(&mut buffer)?;

    buffer.set_color(&ColorSpec::new())?;
    write!(&mut buffer, "Visit ")?;

    buffer.set_color(&green)?;
    write!(&mut buffer, "http://{}:{}/", address_string, port)?;

    buffer.set_color(&ColorSpec::new())?;
    writeln!(&mut buffer, " in your browser for more information.")?;

    writer.print(&buffer)?;

    Ok(())
}
