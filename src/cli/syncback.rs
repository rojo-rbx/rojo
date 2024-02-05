use std::{
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::Context;
use clap::Parser;
use memofs::Vfs;
use rbx_dom_weak::{InstanceBuilder, WeakDom};

use crate::{serve_session::ServeSession, syncback::syncback_loop};

use super::resolve_path;

const UNKNOWN_INPUT_KIND_ERR: &str = "Could not detect what kind of file was inputted. \
                                       Expected input file to end in .rbxl, .rbxlx, .rbxm, or .rbxmx.";

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

        let input_kind = FileKind::from_path(&path_new).context(UNKNOWN_INPUT_KIND_ERR)?;
        let dom_start = Instant::now();
        log::info!("Reading place file at {}", path_new.display());
        let dom_new = read_dom(&path_new, input_kind)?;
        log::info!(
            "Finished opening file in {:0.02}s",
            dom_start.elapsed().as_secs_f32()
        );

        let project_start = Instant::now();
        log::info!("Opening project at {}", path_old.display());
        let session_old = ServeSession::new(Vfs::new_default(), path_old.clone())?;
        log::info!(
            "Finished opening project in {:0.02}s",
            project_start.elapsed().as_secs_f32()
        );

        let dom_old = session_old.tree();

        log::debug!("Old root: {}", dom_old.inner().root().class);
        log::debug!("New root: {}", dom_new.root().class);

        let start = Instant::now();
        log::info!("Beginning syncback...");
        syncback_loop(
            session_old.vfs(),
            &dom_old,
            dom_new,
            session_old.root_project(),
        )?;
        log::info!(
            "Syncback finished in {:.02}s!",
            start.elapsed().as_secs_f32()
        );

        Ok(())
    }
}

fn read_dom(path: &Path, file_kind: FileKind) -> anyhow::Result<WeakDom> {
    let content = fs_err::read(path)?;
    Ok(match file_kind {
        FileKind::Rbxl => rbx_binary::from_reader(content.as_slice())?,
        FileKind::Rbxlx => rbx_xml::from_reader(content.as_slice(), xml_decode_config())?,
        FileKind::Rbxm => {
            let temp_tree = rbx_binary::from_reader(content.as_slice())?;
            let root_children = temp_tree.root().children();
            if root_children.len() != 1 {
                anyhow::bail!(
                    "Rojo does not currently support models with more \
                than one Instance at the Root!"
                );
            }
            let real_root = temp_tree.get_by_ref(root_children[0]).unwrap();
            let mut new_tree = WeakDom::new(InstanceBuilder::new(&real_root.class));
            temp_tree.clone_multiple_into_external(real_root.children(), &mut new_tree);

            new_tree
        }
        FileKind::Rbxmx => {
            let temp_tree = rbx_xml::from_reader(content.as_slice(), xml_decode_config())?;
            let root_children = temp_tree.root().children();
            if root_children.len() != 1 {
                anyhow::bail!(
                    "Rojo does not currently support models with more \
                than one Instance at the Root!"
                );
            }
            let real_root = temp_tree.get_by_ref(root_children[0]).unwrap();
            let mut new_tree = WeakDom::new(InstanceBuilder::new(&real_root.class));
            temp_tree.clone_multiple_into_external(real_root.children(), &mut new_tree);

            new_tree
        }
    })
}

fn xml_decode_config() -> rbx_xml::DecodeOptions<'static> {
    rbx_xml::DecodeOptions::new().property_behavior(rbx_xml::DecodePropertyBehavior::ReadUnknown)
}

/// The different kinds of input that Rojo can syncback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileKind {
    /// An XML model file.
    Rbxmx,

    /// An XML place file.
    Rbxlx,

    /// A binary model file.
    Rbxm,

    /// A binary place file.
    Rbxl,
}

impl FileKind {
    fn from_path(output: &Path) -> Option<FileKind> {
        let extension = output.extension()?.to_str()?;

        match extension {
            "rbxlx" => Some(FileKind::Rbxlx),
            "rbxmx" => Some(FileKind::Rbxmx),
            "rbxl" => Some(FileKind::Rbxl),
            "rbxm" => Some(FileKind::Rbxm),
            _ => None,
        }
    }
}
