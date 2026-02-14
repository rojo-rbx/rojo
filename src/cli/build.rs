use std::{
    io::{BufWriter, Write},
    mem::forget,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use clap::{CommandFactory, Parser};
use fs_err::File;
use memofs::Vfs;
use roblox_install::RobloxStudio;
use tokio::runtime::Runtime;

use crate::{merge_base, serve_session::ServeSession};

use super::resolve_path;

const UNKNOWN_OUTPUT_KIND_ERR: &str = "Could not detect what kind of file to build. \
                                       Expected output file to end in .rbxl, .rbxlx, .rbxm, or .rbxmx.";
const UNKNOWN_PLUGIN_KIND_ERR: &str = "Could not detect what kind of file to build. \
                                       Expected plugin file to end in .rbxm or .rbxmx.";

/// Generates a model or place file from the Rojo project.
#[derive(Debug, Parser)]
pub struct BuildCommand {
    /// Path to the project to build. Defaults to the current directory.
    #[clap(default_value = "")]
    pub project: PathBuf,

    /// Where to output the result.
    ///
    /// Should end in .rbxm, .rbxl, .rbxmx, or .rbxlx.
    #[clap(long, short, conflicts_with = "plugin")]
    pub output: Option<PathBuf>,

    /// Alternative to the output flag that outputs the result in the local plugins folder.
    ///
    /// Should end in .rbxm or .rbxl.
    #[clap(long, short, conflicts_with = "output")]
    pub plugin: Option<PathBuf>,

    /// Path to a base Roblox file (.rbxl, .rbxlx, .rbxm, .rbxmx) to merge
    /// the project into. The Rojo project tree is merged into this file so
    /// that the output contains both the base content and the project's
    /// scripts/configuration.
    #[clap(long)]
    pub base: Option<PathBuf>,

    /// Whether to automatically rebuild when any input files change.
    #[clap(long)]
    pub watch: bool,
}

impl BuildCommand {
    pub fn run(self) -> anyhow::Result<()> {
        let (output_path, output_kind) = match (self.output, self.plugin) {
            (None, None) => {
                BuildCommand::command()
                    .error(
                        clap::ErrorKind::MissingRequiredArgument,
                        "one of the following arguments must be provided: \n    --output <OUTPUT>\n    --plugin <PLUGIN>",
                    )
                    .exit();
            }
            (Some(output), None) => {
                let output_kind =
                    OutputKind::from_output_path(&output).context(UNKNOWN_OUTPUT_KIND_ERR)?;

                (output, output_kind)
            }
            (None, Some(plugin)) => {
                if plugin.is_absolute() {
                    bail!("plugin flag path cannot be absolute.")
                }

                let output_kind =
                    OutputKind::from_plugin_path(&plugin).context(UNKNOWN_PLUGIN_KIND_ERR)?;
                let studio = RobloxStudio::locate()?;

                (studio.plugins_path().join(&plugin), output_kind)
            }
            _ => unreachable!(),
        };

        let project_path = resolve_path(&self.project);

        log::trace!("Constructing in-memory filesystem");
        let vfs = Vfs::new_default();
        vfs.set_watch_enabled(self.watch);

        let session = ServeSession::new(vfs, project_path)?;
        let mut cursor = session.message_queue().cursor();

        write_model(&session, &output_path, output_kind, self.base.as_deref())?;

        if self.watch {
            let rt = Runtime::new().unwrap();

            loop {
                let receiver = session.message_queue().subscribe(cursor);
                let (new_cursor, _patch_set) = rt.block_on(receiver).unwrap();
                cursor = new_cursor;

                write_model(&session, &output_path, output_kind, self.base.as_deref())?;
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

impl OutputKind {
    fn from_output_path(output: &Path) -> Option<OutputKind> {
        let extension = output.extension()?.to_str()?;

        match extension {
            "rbxlx" => Some(OutputKind::Rbxlx),
            "rbxmx" => Some(OutputKind::Rbxmx),
            "rbxl" => Some(OutputKind::Rbxl),
            "rbxm" => Some(OutputKind::Rbxm),
            _ => None,
        }
    }

    fn from_plugin_path(output: &Path) -> Option<OutputKind> {
        let extension = output.extension()?.to_str()?;

        match extension {
            "rbxmx" => Some(OutputKind::Rbxmx),
            "rbxm" => Some(OutputKind::Rbxm),
            _ => None,
        }
    }
}

fn xml_encode_config() -> rbx_xml::EncodeOptions<'static> {
    rbx_xml::EncodeOptions::new().property_behavior(rbx_xml::EncodePropertyBehavior::WriteUnknown)
}

#[profiling::function]
fn write_model(
    session: &ServeSession,
    output: &Path,
    output_kind: OutputKind,
    base_path: Option<&Path>,
) -> anyhow::Result<()> {
    println!("Building project '{}'", session.project_name());

    let tree = session.tree();

    log::trace!("Opening output file for write");
    let mut file = BufWriter::new(File::create(output)?);

    if let Some(base_path) = base_path {
        let base_path = super::resolve_path(base_path);

        log::trace!("Reading base file: {}", base_path.display());
        let base_dom = merge_base::read_base_dom(&base_path)?;

        log::trace!("Merging Rojo project into base file");
        let merged_dom = merge_base::merge_rojo_into_base(base_dom, &tree)?;

        // The merged DOM's root is always the WeakDom synthetic root
        // (DataModel). For both place and model output, we encode its
        // children — the services/instances — not the root itself.
        let encode_ids = merged_dom.root().children().to_vec();

        match output_kind {
            OutputKind::Rbxl | OutputKind::Rbxm => {
                rbx_binary::to_writer(&mut file, &merged_dom, &encode_ids)?;
            }
            OutputKind::Rbxlx | OutputKind::Rbxmx => {
                rbx_xml::to_writer(&mut file, &merged_dom, &encode_ids, xml_encode_config())?;
            }
        }
    } else {
        let root_id = tree.get_root_id();

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

                rbx_xml::to_writer(
                    &mut file,
                    tree.inner(),
                    top_level_ids,
                    xml_encode_config(),
                )?;
            }
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
