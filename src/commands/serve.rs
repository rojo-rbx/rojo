use std::{
    io::{self, Write},
    path::PathBuf,
    sync::Arc,
};

use failure::Fail;
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

use crate::{
    imfs::{Imfs, RealFetcher, WatchMode},
    project::{Project, ProjectLoadError},
    serve_session::ServeSession,
    web::LiveServer,
};

const DEFAULT_PORT: u16 = 34872;

#[derive(Debug)]
pub struct ServeOptions {
    pub fuzzy_project_path: PathBuf,
    pub port: Option<u16>,
}

#[derive(Debug, Fail)]
pub enum ServeError {
    #[fail(display = "Couldn't load project: {}", _0)]
    ProjectLoad(#[fail(cause)] ProjectLoadError),
}

impl_from!(ServeError {
    ProjectLoadError => ProjectLoad,
});

pub fn serve(options: &ServeOptions) -> Result<(), ServeError> {
    let maybe_project = match Project::load_fuzzy(&options.fuzzy_project_path) {
        Ok(project) => Some(project),
        Err(ProjectLoadError::NotFound) => None,
        Err(other) => return Err(other.into()),
    };

    let port = options
        .port
        .or_else(|| {
            maybe_project
                .as_ref()
                .and_then(|project| project.serve_port)
        })
        .unwrap_or(DEFAULT_PORT);

    let _ = show_start_message(port);

    let imfs = Imfs::new(RealFetcher::new(WatchMode::Enabled));

    let session = Arc::new(ServeSession::new(
        imfs,
        &options.fuzzy_project_path,
        maybe_project,
    ));

    let server = LiveServer::new(session);

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

    writeln!(&mut buffer, "")?;

    buffer.set_color(&ColorSpec::new())?;
    write!(&mut buffer, "Visit ")?;

    buffer.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
    write!(&mut buffer, "http://localhost:{}/", port)?;

    buffer.set_color(&ColorSpec::new())?;
    writeln!(&mut buffer, " in your browser for more information.")?;

    writer.print(&buffer)?;

    Ok(())
}
