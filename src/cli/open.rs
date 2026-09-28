use std::{path::PathBuf, process::Command, sync::Arc};

use anyhow::Context;
use clap::{ArgGroup, Parser};
use memofs::Vfs;
use roblox_install::RobloxStudio;

use crate::{
    cli::{
        build::{write_model, OutputKind, UNKNOWN_PLACE_KIND_ERR},
        resolve_path,
        serve::{show_start_message, ServerOptions},
        GlobalOptions,
    },
    serve_session::ServeSession,
    web::LiveServer,
};

/// Serve a project and open a place in Roblox Studio that connects to it.
#[derive(Debug, Parser)]
#[clap(group(
    ArgGroup::new("target").required(true).args(&["place", "output", "place-id"])
))]
pub struct OpenCommand {
    /// Path to the project to serve. Defaults to the current directory.
    #[clap(default_value = "")]
    pub project: PathBuf,

    #[clap(flatten)]
    pub server: ServerOptions,

    /// An existing place file to open. Must end in `.rbxl` or `.rbxlx`.
    #[clap(long)]
    pub place: Option<PathBuf>,

    /// Build the project to this place file, then open it. Must end in `.rbxl`
    /// or `.rbxlx`.
    #[clap(long)]
    pub output: Option<PathBuf>,

    /// The ID of a published place to open. Requires `--universe-id`.
    #[clap(long, requires = "universe-id")]
    pub place_id: Option<u64>,

    /// The universe ID of the place given with `--place-id`.
    #[clap(long, requires = "place-id")]
    pub universe_id: Option<u64>,
}

impl OpenCommand {
    pub fn run(self, global: GlobalOptions) -> anyhow::Result<()> {
        let project_path = resolve_path(&self.project)?;
        let studio = RobloxStudio::locate()?;
        let vfs = Vfs::new_default()?;
        let session = Arc::new(ServeSession::new(vfs, project_path)?);
        let (ip, port, allowed_hosts) = self.server.resolve_options(&session);
        let script_path =
            std::env::temp_dir().join(format!("rojo-open-{}.luau", std::process::id()));

        let mut command = Command::new(studio.application_path());
        command
            .arg("--task")
            .arg("RunScript")
            .arg("--runScriptFile")
            .arg(&script_path);

        if let Some(output) = self.output {
            let output_kind =
                OutputKind::from_place_path(&output).context(UNKNOWN_PLACE_KIND_ERR)?;

            write_model(&session, &output, output_kind)?;

            command.arg("--localPlaceFile").arg(output);
        } else if let Some(place) = self.place {
            OutputKind::from_place_path(&place).context(UNKNOWN_PLACE_KIND_ERR)?;

            let place = dunce::canonicalize(&place)
                .with_context(|| format!("Could not find place file {}", place.display()))?;

            command.arg("--localPlaceFile").arg(place);
        } else if let (Some(place_id), Some(universe_id)) = (self.place_id, self.universe_id) {
            command
                .arg("--universeId")
                .arg(universe_id.to_string())
                .arg("--placeId")
                .arg(place_id.to_string());
        }

        let session_id = session.session_id();
        fs_err::write(
            &script_path,
            format!(
                r#"local value = Instance.new("Configuration")
value.Name = `ROJO_OPEN_{{game:GetService("StudioService"):GetUserId()}}`
value.Archivable = false
value:SetAttribute("Host", "{ip}")
value:SetAttribute("Port", "{port}")
value:SetAttribute("SessionId", "{session_id}")
value.Parent = game"#,
            ),
        )?;

        let server = LiveServer::new(session);

        server.start((ip, port).into(), allowed_hosts, || {
            let _ = show_start_message(ip, port, global.color.into());

            // Wait for a successful bind so the plugin can't connect to another server on this address.
            if let Err(err) = command.spawn() {
                log::error!("Could not launch Roblox Studio: {err}");
            }
        })?;

        Ok(())
    }
}
