use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use clap::Parser;
use memofs::Vfs;
use rbx_dom_weak::WeakDom;
use rbx_xml::DecodeOptions;

use crate::{serve_session::ServeSession, syncback::syncback_loop};

use super::resolve_path;

/// Performs syncback for a project file
#[derive(Debug, Parser)]
pub struct SyncbackCommand {
    /// Path to the project to sync back to.
    #[clap(default_value = "")]
    pub project: PathBuf,

    /// Path to the place to perform syncback on.
    #[clap(long, short)]
    pub input: PathBuf,
}

impl SyncbackCommand {
    pub fn run(&self) -> anyhow::Result<()> {
        let path_old = resolve_path(&self.project);
        let path_new = resolve_path(&self.input);

        let project_start = Instant::now();
        log::info!("Opening project at {}", path_old.display());
        let session_old = ServeSession::new(Vfs::new_default(), path_old.clone())?;
        log::info!(
            "Finished opening project in {:0.02}s",
            project_start.elapsed().as_secs_f32()
        );

        let dom_old = session_old.tree();

        let dom_start = Instant::now();
        log::info!("Reading place file at {}", path_new.display());
        let dom_new = read_dom(&path_new);
        log::info!(
            "Finished opening file in {:0.02}s",
            dom_start.elapsed().as_secs_f32()
        );

        let start = Instant::now();
        log::info!("Beginning syncback...");
        syncback_loop(
            session_old.vfs(),
            &dom_old,
            &dom_new,
            session_old.root_project(),
        )?;
        log::info!(
            "Syncback finished in {:.02}s!",
            start.elapsed().as_secs_f32()
        );

        Ok(())
    }
}

fn read_dom(path: &Path) -> WeakDom {
    let content = fs::read(path).unwrap();
    if &content[0..8] == b"<roblox!" {
        log::debug!("Reading {} as a binary file", path.display());
        rbx_binary::from_reader(content.as_slice()).unwrap()
    } else if &content[0..7] == b"<roblox" {
        log::debug!("Reading {} as an xml file", path.display());
        rbx_xml::from_reader(
            content.as_slice(),
            DecodeOptions::new().property_behavior(rbx_xml::DecodePropertyBehavior::ReadUnknown),
        )
        .unwrap()
    } else {
        panic!("invalid Roblox file at {}", path.display())
    }
}
