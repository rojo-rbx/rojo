use std::{
    io::{self, Write},
    net::IpAddr,
    net::Ipv4Addr,
    sync::Arc,
};

use anyhow::Result;
use memofs::Vfs;
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

use crate::{
    cli::{GlobalOptions, ServeCommand},
    serve_session::ServeSession,
    web::LiveServer,
};

const DEFAULT_BIND_ADDRESS: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);
const DEFAULT_PORT: u16 = 34872;

pub fn serve(global: GlobalOptions, options: ServeCommand) -> Result<()> {
    let vfs = Vfs::new_default();

    let session = Arc::new(ServeSession::new(vfs, &options.absolute_project())?);

    let ip = options.address.unwrap_or(DEFAULT_BIND_ADDRESS.into());

    let port = options
        .port
        .or_else(|| session.project_port())
        .unwrap_or(DEFAULT_PORT);

    let server = LiveServer::new(session);

    let _ = show_start_message(ip, port, global.color.into());
    server.start((ip, port).into());

    Ok(())
}

fn show_start_message(bind_address: IpAddr, port: u16, color: ColorChoice) -> io::Result<()> {
    let writer = BufferWriter::stdout(color);
    let mut buffer = writer.buffer();

    writeln!(&mut buffer, "Rojo server listening:")?;

    write!(&mut buffer, "  Address: ")?;
    buffer.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;

    if bind_address.is_loopback() {
        writeln!(&mut buffer, "localhost")?;
    } else {
        writeln!(&mut buffer, "{}", bind_address)?;
    }

    buffer.set_color(&ColorSpec::new())?;
    write!(&mut buffer, "  Port:    ")?;
    buffer.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
    writeln!(&mut buffer, "{}", port)?;

    writeln!(&mut buffer)?;

    buffer.set_color(&ColorSpec::new())?;
    write!(&mut buffer, "Visit ")?;

    buffer.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
    write!(&mut buffer, "http://localhost:{}/", port)?;

    buffer.set_color(&ColorSpec::new())?;
    writeln!(&mut buffer, " in your browser for more information.")?;

    writer.print(&buffer)?;

    Ok(())
}
