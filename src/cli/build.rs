use std::{
    io::{BufWriter, Write},
    mem::forget,
    path::{Path, PathBuf},
};

use anyhow::Context;
use clap::Parser;
use fs_err::File;
use memofs::Vfs;
use tokio::runtime::Runtime;

use crate::serve_session::ServeSession;

use super::resolve_path;

const UNKNOWN_OUTPUT_KIND_ERR: &str = "Could not detect what kind of file to build. \
                                       Expected output file to end in .rbxl, .rbxlx, .rbxm, or .rbxmx.";

/// Generates a model or place file from the Rojo project.
#[derive(Debug, Parser)]
pub struct BuildCommand {
    /// Path to the project to serve. Defaults to the current directory.
    #[clap(default_value = "")]
    pub project: PathBuf,

    /// Where to output the result.
    ///
    /// Should end in .rbxm, .rbxl, .rbxmx, or .rbxlx.
    #[clap(long, short)]
    pub output: PathBuf,

    /// Whether to automatically rebuild when any input files change.
    #[clap(long)]
    pub watch: bool,
}

impl BuildCommand {
    pub fn run(self) -> anyhow::Result<()> {
        let project_path = resolve_path(&self.project);

        let output_kind = detect_output_kind(&self.output).context(UNKNOWN_OUTPUT_KIND_ERR)?;

        log::trace!("Constructing in-memory filesystem");
        let vfs = Vfs::new_default();
        vfs.set_watch_enabled(self.watch);

        let session = ServeSession::new(vfs, &project_path)?;
        let mut cursor = session.message_queue().cursor();

        write_model(&session, &self.output, output_kind)?;

        if self.watch {
            let rt = Runtime::new().unwrap();

            loop {
                let receiver = session.message_queue().subscribe(cursor);
                let (new_cursor, _patch_set) = rt.block_on(receiver).unwrap();
                cursor = new_cursor;

                write_model(&session, &self.output, output_kind)?;
            }
        }

        // Avoid dropping ServeSession: it's potentially VERY expensive to drop
        // and we're about to exit anyways.
        forget(session);

        Ok(())
    }
}

/// The different kinds of output that Rojo can build to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputKind {
    /// An XML model file.
    Rbxmx,

    /// An XML place file.
    Rbxlx,

    /// A binary model file.
    Rbxm,

    /// A binary place file.
    Rbxl,
}

fn detect_output_kind(output: &Path) -> Option<OutputKind> {
    let extension = output.extension()?.to_str()?;

    match extension {
        "rbxlx" => Some(OutputKind::Rbxlx),
        "rbxmx" => Some(OutputKind::Rbxmx),
        "rbxl" => Some(OutputKind::Rbxl),
        "rbxm" => Some(OutputKind::Rbxm),
        _ => None,
    }
}

fn xml_encode_config() -> rbx_xml::EncodeOptions {
    rbx_xml::EncodeOptions::new().property_behavior(rbx_xml::EncodePropertyBehavior::WriteUnknown)
}

#[profiling::function]
fn write_model(
    session: &ServeSession,
    output: &Path,
    output_kind: OutputKind,
) -> anyhow::Result<()> {
    println!("Building project '{}'", session.project_name());

    let tree = session.tree();
    let root_id = tree.get_root_id();

    log::trace!("Opening output file for write");
    let mut file = BufWriter::new(File::create(output)?);

    match output_kind {
        OutputKind::Rbxm => {
            rbx_binary::to_writer(&mut file, tree.inner(), &[root_id])?;
        }
        OutputKind::Rbxl => {
            let root_instance = tree.get_instance(root_id).unwrap();
            let top_level_ids = root_instance.children();

            rbx_binary::to_writer(&mut file, tree.inner(), top_level_ids)?;
        }
        OutputKind::Rbxmx => {
            // Model files include the root instance of the tree and all its
            // descendants.

            rbx_xml::to_writer(&mut file, tree.inner(), &[root_id], xml_encode_config())?;
        }
        OutputKind::Rbxlx => {
            // Place files don't contain an entry for the DataModel, but our
            // WeakDom representation does.

            let root_instance = tree.get_instance(root_id).unwrap();
            let top_level_ids = root_instance.children();

            rbx_xml::to_writer(&mut file, tree.inner(), top_level_ids, xml_encode_config())?;
        }
    }

    file.flush()?;

    let filename = output
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("<invalid utf-8>");
    println!("Built project to {}", filename);

    Ok(())
}
