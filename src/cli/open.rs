use anyhow::{Context, Ok};
use clap::Parser;
use memofs::Vfs;
use roblox_install::RobloxStudio;

use crate::{
    cli::{
        build::{detect_output_kind, write_model, UNKNOWN_OUTPUT_KIND_ERR},
        plugin::install_plugin,
        serve::{show_start_message, DEFAULT_BIND_ADDRESS, DEFAULT_PORT},
    },
    serve_session::ServeSession,
    web::LiveServer,
    PROJECT_FILENAME,
};
use std::{env, net::IpAddr, path::PathBuf, sync::Arc};

use super::GlobalOptions;

#[derive(Debug, Parser)]
pub struct OpenCommand {
    /// Path to the project file to serve from. Defaults to default.project.json.
    #[clap(value_parser)]
    pub project: Option<PathBuf>,

    // Path to an output place to build and serve to. Will be created automatically
    /// if it doesn't exist.
    #[clap(long)]
    pub output: PathBuf,

    /// The IP address to listen on. Defaults to `127.0.0.1`.
    #[clap(long)]
    pub address: Option<IpAddr>,

    /// The port to listen on. Defaults to the project's preference, or `34872` if
    /// it has none.
    #[clap(long)]
    pub port: Option<u16>,
}

impl OpenCommand {
    pub fn run(self, global: GlobalOptions) -> anyhow::Result<()> {
        let project = self
            .project
            .unwrap_or_else(|| env::current_dir().unwrap().join(PROJECT_FILENAME));
        let output_kind = detect_output_kind(&self.output).context(UNKNOWN_OUTPUT_KIND_ERR)?;

        log::trace!("Constructing in-memory filesystem");
        let vfs = Vfs::new_default();
        let session = ServeSession::new(vfs, &project)?;

        if !self.output.exists() {
            write_model(&session, &self.output, output_kind)?;
        }
        install_plugin().unwrap();

        opener::open(&self.output).expect("Could not open place in Roblox Studio");

        let ip = self
            .address
            .or_else(|| session.serve_address())
            .unwrap_or_else(|| DEFAULT_BIND_ADDRESS.into());
        let port = self
            .port
            .or_else(|| session.project_port())
            .unwrap_or(DEFAULT_PORT);
        let server = LiveServer::new(Arc::new(session));

        show_start_message(ip, port, global.color.into())?;
        server.start((ip, port).into());

        Ok(())
    }
}
