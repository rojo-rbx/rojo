use std::{
    fs::File,
    io::{BufReader, BufWriter},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener},
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context;
use clap::Parser;
use memofs::Vfs;
use rbx_dom_weak::InstanceBuilder;

use crate::{serve_session::ServeSession, web::LiveServer};

use super::{
    build::{write_model, OutputKind},
    resolve_path,
    serve::show_start_message,
    GlobalOptions,
};

const UNKNOWN_OUTPUT_KIND_ERR: &str = "Could not detect what kind of file to build. \
                                       Expected output file to end in .rbxl or .rbxlx.";
const DEFAULT_BIND_ADDRESS: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);
const DEFAULT_PORT: u16 = 34872;

/// TODO
#[derive(Debug, Parser)]
pub struct OpenCommand {
    /// Path to the project to serve. Defaults to the current directory.
    #[clap(default_value = "")]
    pub project: PathBuf,

    /// Where to output the result.
    ///
    /// Should end in .rbxm, .rbxl.
    #[clap(long, short)]
    pub output: PathBuf,

    /// The IP address to listen on. Defaults to `127.0.0.1`.
    #[clap(long)]
    pub address: Option<IpAddr>,

    /// The port to listen on. Defaults to the project's preference, or a random usable port if
    /// it has none.
    #[clap(long)]
    pub port: Option<u16>,
}

// TODO: Support existing places.
impl OpenCommand {
    pub fn run(self, global: GlobalOptions) -> anyhow::Result<()> {
        let project = resolve_path(&self.project);

        let vfs = Vfs::new_default();
        let session = ServeSession::new(vfs, project)?;

        let ip = self
            .address
            .or_else(|| session.serve_address())
            .unwrap_or(DEFAULT_BIND_ADDRESS.into());

        let port = self
            .port
            .or_else(|| session.project_port())
            .or_else(|| random_port(ip))
            .unwrap_or(DEFAULT_PORT);

        let output_kind =
            OutputKind::from_output_path_place(&self.output).context(UNKNOWN_OUTPUT_KIND_ERR)?;

        write_model(&session, &self.output, OutputKind::Rbxl)?;

        inject_rojo_open_string_value(&self.output, output_kind, ip, port)?;

        opener::open(self.output)?;

        let server = LiveServer::new(Arc::new(session));

        let _ = show_start_message(ip, port, global.color.into());
        server.start((ip, port).into());

        Ok(())
    }
}

fn random_port(ip: IpAddr) -> Option<u16> {
    Some(
        TcpListener::bind(SocketAddr::new(ip, 0))
            .ok()?
            .local_addr()
            .ok()?
            .port(),
    )
}

fn inject_rojo_open_string_value(
    path: &Path,
    output_kind: OutputKind,
    ip: IpAddr,
    port: u16,
) -> anyhow::Result<()> {
    let file = File::open(path).unwrap();

    let mut dom = match output_kind {
        OutputKind::Rbxl => rbx_binary::from_reader(BufReader::new(file)).unwrap(),
        OutputKind::Rbxlx => rbx_xml::from_reader_default(BufReader::new(file)).unwrap(),
        _ => unreachable!(),
    };

    let ip = if ip.is_loopback() {
        "localhost".to_string()
    } else {
        ip.to_string()
    };

    dom.insert(
        dom.root_ref(),
        InstanceBuilder::new("StringValue")
            .with_name("ROJO_OPEN")
            .with_property("Value", format!("{ip},{port}",)),
    );

    let root_instance = dom.root();
    let top_level_ids = root_instance.children();
    let output = BufWriter::new(File::create(path).unwrap());

    match output_kind {
        OutputKind::Rbxl => rbx_binary::to_writer(output, &dom, top_level_ids)?,
        OutputKind::Rbxlx => rbx_xml::to_writer_default(output, &dom, top_level_ids)?,
        _ => unreachable!(),
    }

    Ok(())
}
