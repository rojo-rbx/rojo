use std::{
    io::{self, Write as _},
    mem::forget,
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::Context;
use clap::Parser;
use fs_err::File;
use memofs::Vfs;
use rbx_dom_weak::{InstanceBuilder, WeakDom};
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

use crate::{
    serve_session::ServeSession,
    syncback::{syncback_loop, FsSnapshot},
};

use super::{resolve_path, GlobalOptions};

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

    /// If provided, syncback will list all of the changes it will make to the
    /// file system before making them.
    #[clap(long, short)]
    pub list: bool,

    /// If provided, syncback will not actually write anything to the file
    /// system.
    #[clap(long)]
    pub dry_run: bool,

    /// If provided, the prompt for writing to the file system is skipped.
    #[clap(long, short = 'y')]
    pub non_interactive: bool,
}

impl SyncbackCommand {
    pub fn run(&self, global: GlobalOptions) -> anyhow::Result<()> {
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

        let vfs = Vfs::new_default();
        vfs.set_watch_enabled(false);

        let project_start = Instant::now();
        log::info!("Opening project at {}", path_old.display());
        let session_old = ServeSession::new(vfs, path_old.clone())?;
        log::info!(
            "Finished opening project in {:0.02}s",
            project_start.elapsed().as_secs_f32()
        );

        let mut dom_old = session_old.tree();

        log::debug!("Old root: {}", dom_old.inner().root().class);
        log::debug!("New root: {}", dom_new.root().class);

        let start = Instant::now();
        log::info!("Beginning syncback...");
        let snapshot = syncback_loop(
            session_old.vfs(),
            &mut dom_old,
            dom_new,
            session_old.root_project(),
        )?;
        log::info!(
            "Syncback finished in {:.02}s!",
            start.elapsed().as_secs_f32()
        );

        let base_path = session_old.root_project().folder_location();
        if self.list {
            list_files(&snapshot, global.color.into(), base_path)?;
        }

        if !self.dry_run {
            if !self.non_interactive {
                println!(
                    "Would write {} files/folders and remove {} files/folders.",
                    snapshot.added_paths().len(),
                    snapshot.removed_paths().len()
                );
                print!("Is this okay? (Y/N): ");
                io::stdout().flush()?;
                let mut line = String::with_capacity(1);
                io::stdin().read_line(&mut line)?;
                line = line.trim().to_lowercase();
                if line != "y" {
                    println!("Aborting due to user input!");
                    return Ok(());
                }
            }
            log::info!("Writing to the file system...");
            snapshot.write_to_vfs(base_path, session_old.vfs())?;
        } else {
            println!(
                "Would write {} files/folders and remove {} files/folders.",
                snapshot.added_paths().len(),
                snapshot.removed_paths().len()
            );
            println!("Aborting before writing to file system due to `--dry-run`");
        }

        // It is potentially prohibitively expensive to drop a ServeSession,
        // and the program is about to exit anyway so we're just going to forget
        // about it.
        drop(dom_old);
        forget(session_old);

        Ok(())
    }
}

fn read_dom(path: &Path, file_kind: FileKind) -> anyhow::Result<WeakDom> {
    let content = File::open(path)?;
    Ok(match file_kind {
        FileKind::Rbxl => rbx_binary::from_reader(content)?,
        FileKind::Rbxlx => rbx_xml::from_reader(content, xml_decode_config())?,
        FileKind::Rbxm => {
            let temp_tree = rbx_binary::from_reader(content)?;
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
            let temp_tree = rbx_xml::from_reader(content, xml_decode_config())?;
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

fn list_files(snapshot: &FsSnapshot, color: ColorChoice, base_path: &Path) -> io::Result<()> {
    let no_color = ColorSpec::new();
    let mut add_color = ColorSpec::new();
    add_color.set_fg(Some(Color::Green));
    let mut remove_color = ColorSpec::new();
    remove_color.set_fg(Some(Color::Red));

    // We emit this to stderr because otherwise it'd be impossible
    // to pipe it separately from normal output.
    let writer = BufferWriter::stderr(color);
    let mut buffer = writer.buffer();

    if snapshot.is_empty() {
        writeln!(&mut buffer, "No files/added would be removed or added.")?;
    } else {
        let added = snapshot.added_paths();
        if !added.is_empty() {
            writeln!(&mut buffer, "Writing files/folders:")?;
            buffer.set_color(&add_color)?;
            for path in added {
                writeln!(
                    &mut buffer,
                    "{}",
                    path.strip_prefix(base_path).unwrap_or(path).display()
                )?;
            }
            buffer.set_color(&no_color)?;
        }
        let removed = snapshot.removed_paths();
        if !removed.is_empty() {
            writeln!(&mut buffer, "Removing files/folders:")?;
            buffer.set_color(&remove_color)?;
            for path in removed {
                writeln!(
                    &mut buffer,
                    "{}",
                    path.strip_prefix(base_path).unwrap_or(path).display()
                )?;
            }
        }
        buffer.set_color(&no_color)?;
    }

    writer.print(&buffer)
}
