use std::path::PathBuf;

use anyhow::bail;
use clap::Parser;
use memofs::Vfs;

use crate::{serve_session::ServeSession, web::external};

use super::resolve_path;

/// Builds the project and uploads it to Roblox.
#[derive(Debug, Parser)]
pub struct UploadCommand {
    /// Path to the project to upload. Defaults to the current directory.
    #[clap(default_value = "")]
    pub project: PathBuf,

    /// Authenication cookie to use.
    /// If not specified, Rojo will attempt to find one from the system
    /// automatically.
    #[clap(long)]
    pub cookie: Option<String>,

    /// API key obtained from create.roblox.com/dashboard/credentials.
    /// Rojo will use the Open Cloud API when this is provided.
    ///
    /// This API key must have `asset:read` and `asset:write` permissions.
    #[clap(long = "api-key", alias = "api_key")]
    pub api_key: Option<String>,

    /// The Universe ID of the given place. Required when using the Open Cloud API.
    ///
    /// No longer used, but supported for compatibility.
    #[clap(hidden = true, long = "universe_id", alias = "universe_id")]
    pub universe_id: Option<u64>,

    /// Asset ID to upload to.
    #[clap(long = "asset-id", alias = "asset_id")]
    pub asset_id: u64,

    /// If passed, publishing will be skipped for an uploaded place.
    /// Does nothing for models and other asset types.
    #[clap(long = "skip-publish", alias = "skip_publish")]
    pub skip_publish: bool,
}

impl UploadCommand {
    pub fn run(self) -> anyhow::Result<()> {
        if self.universe_id.is_some() {
            log::warn!("--universe_id is no longer required to upload places")
        }

        let project_path = resolve_path(&self.project)?;

        let vfs = Vfs::new_default()?;

        let session = ServeSession::new(vfs, project_path)?;

        let tree = session.tree();
        let inner_tree = tree.inner();
        let root = inner_tree.root();

        let encode_ids = match root.class.as_str() {
            "DataModel" => root.children().to_vec(),
            _ => vec![root.referent()],
        };

        let mut buffer = Vec::new();

        log::trace!("Encoding binary model");
        rbx_binary::to_writer(&mut buffer, tree.inner(), &encode_ids)?;

        match (self.cookie, self.api_key) {
            (None, Some(api_key)) => external::patch_asset_open_cloud(
                &api_key,
                self.asset_id,
                buffer,
                !self.skip_publish,
            )?,
            (Some(cookie), None) => {
                external::patch_asset_cookie(&cookie, self.asset_id, buffer, !self.skip_publish)?
            }
            (None, None) => {
                if let Some(cookie) = rbx_cookie::get_value() {
                    external::patch_asset_cookie(
                        &cookie,
                        self.asset_id,
                        buffer,
                        !self.skip_publish,
                    )?
                } else {
                    bail!("A Roblox cookie could not be found on your system. You must provide either --cookie or --api-key.")
                }
            }
            (Some(_), Some(_)) => {
                bail!("You must specify either --cookie or --api-key to upload, not both")
            }
        };

        Ok(())
    }
}
