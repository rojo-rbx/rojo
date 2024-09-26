use std::{
    io::{self, BufReader, Write as _},
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

/// Performs 'syncback' for the provided project, using the `input` file
/// given.
///
/// Syncback exists to convert Roblox files into a Rojo project automatically.
/// It uses the project.json file provided to traverse the Roblox file passed as
/// to serialize Instances to the file system in a format that Rojo understands.
#[derive(Debug, Parser)]
pub struct SyncbackCommand {
    /// Path to the project to sync back to.
    #[clap(default_value = "")]
    pub project: PathBuf,

    /// Path to the Roblox file to pull Instances from.
    #[clap(long, short)]
    pub input: PathBuf,

    /// If provided, a list all of the files and directories that will be
    /// added or removed is emitted.
    #[clap(long, short)]
    pub list: bool,

    /// If provided, syncback will not actually write anything to the file
    /// system. The command will otherwise run normally.
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
        let dom_start_timer = Instant::now();
        let dom_new = read_dom(&path_new, input_kind)?;
        log::debug!(
            "Finished opening file in {:0.02}s",
            dom_start_timer.elapsed().as_secs_f32()
        );

        let vfs = Vfs::new_default();
        vfs.set_watch_enabled(false);

        let project_start_timer = Instant::now();
        let session_old = ServeSession::new(vfs, path_old.clone())?;
        log::debug!(
            "Finished opening project in {:0.02}s",
            project_start_timer.elapsed().as_secs_f32()
        );

        let mut dom_old = session_old.tree();

        log::debug!("Old root: {}", dom_old.inner().root().class);
        log::debug!("New root: {}", dom_new.root().class);

        if log::log_enabled!(log::Level::Trace) {
            log::trace!("Children of old root:");
            for child in dom_old.inner().root().children() {
                let inst = dom_old.get_instance(*child).unwrap();
                log::trace!("{} (class: {})", inst.name(), inst.class_name());
            }
            log::trace!("Children of new root:");
            for child in dom_new.root().children() {
                let inst = dom_new.get_by_ref(*child).unwrap();
                log::trace!("{} (class: {})", inst.name, inst.class);
            }
        }

        let syncback_timer = Instant::now();
        println!("Beginning syncback...");
        let snapshot = syncback_loop(
            session_old.vfs(),
            &mut dom_old,
            dom_new,
            session_old.root_project(),
        )?;
        log::debug!(
            "Syncback finished in {:.02}s!",
            syncback_timer.elapsed().as_secs_f32()
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
            println!("Writing to the file system...");
            snapshot.write_to_vfs(base_path, session_old.vfs())?;
            println!("Finished syncback.")
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
    let content = BufReader::new(File::open(path)?);
    match file_kind {
        FileKind::Rbxl => rbx_binary::from_reader(content).with_context(|| {
            format!(
                "Could not deserialize binary place file at {}",
                path.display()
            )
        }),
        FileKind::Rbxlx => rbx_xml::from_reader(content, xml_decode_config())
            .with_context(|| format!("Could not deserialize XML place file at {}", path.display())),
        FileKind::Rbxm => {
            let temp_tree = rbx_binary::from_reader(content).with_context(|| {
                format!(
                    "Could not deserialize binary place file at {}",
                    path.display()
                )
            })?;

            process_model_dom(temp_tree)
        }
        FileKind::Rbxmx => {
            let temp_tree =
                rbx_xml::from_reader(content, xml_decode_config()).with_context(|| {
                    format!("Could not deserialize XML model file at {}", path.display())
                })?;
            process_model_dom(temp_tree)
        }
    }
}

fn process_model_dom(dom: WeakDom) -> anyhow::Result<WeakDom> {
    let temp_children = dom.root().children();
    if temp_children.len() == 1 {
        let real_root = dom.get_by_ref(temp_children[0]).unwrap();
        let mut new_tree = WeakDom::new(InstanceBuilder::new(&real_root.class));
        for (name, property) in &real_root.properties {
            new_tree
                .root_mut()
                .properties
                .insert(name.to_owned(), property.to_owned());
        }

        let children = dom.clone_multiple_into_external(real_root.children(), &mut new_tree);
        for child in children {
            new_tree.transfer_within(child, new_tree.root_ref());
        }
        Ok(new_tree)
    } else {
        anyhow::bail!(
            "Rojo does not currently support models with more \
        than one Instance at the Root!"
        );
    }
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
        writeln!(
            &mut buffer,
            "No files/directories would be removed or added."
        )?;
    } else {
        let added = snapshot.added_paths();
        if !added.is_empty() {
            writeln!(&mut buffer, "Writing files/directories:")?;
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
            writeln!(&mut buffer, "Removing files/directories:")?;
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
