use std::{
    io::{self, Write},
    sync::Arc,
};

use failure::Fail;
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

use crate::{
    cli::ServeCommand,
    project::ProjectLoadError,
    serve_session::ServeSession,
    vfs::{RealFetcher, Vfs, WatchMode},
    web::LiveServer,
};

const DEFAULT_PORT: u16 = 34872;

#[derive(Debug, Fail)]
pub enum ServeError {
    #[fail(display = "Couldn't load project: {}", _0)]
    ProjectLoad(#[fail(cause)] ProjectLoadError),
}

impl_from!(ServeError {
    ProjectLoadError => ProjectLoad,
});

pub fn serve(options: ServeCommand) -> Result<(), ServeError> {
    let vfs = Vfs::new(RealFetcher::new(WatchMode::Enabled));

    let session = Arc::new(ServeSession::new(vfs, &options.project));

    let port = options
        .port
        .or_else(|| session.project_port())
        .unwrap_or(DEFAULT_PORT);

    let server = LiveServer::new(session);

    let _ = show_start_message(port);
    server.start(port);

    Ok(())
}

fn show_start_message(port: u16) -> io::Result<()> {
    let writer = BufferWriter::stdout(ColorChoice::Auto);
    let mut buffer = writer.buffer();

    writeln!(&mut buffer, "Rojo server listening:")?;

    write!(&mut buffer, "  Address: ")?;
    buffer.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
    writeln!(&mut buffer, "localhost")?;

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
