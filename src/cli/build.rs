use std::{
    io::{BufWriter, Write},
    mem::forget,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context};
use clap::{CommandFactory, Parser};
use memofs::Vfs;
use roblox_install::RobloxStudio;
use tokio::runtime::Runtime;

use crate::serve_session::ServeSession;

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

        let project_path = resolve_path(&self.project)?;

        log::trace!("Constructing in-memory filesystem");
        let vfs = Vfs::new_default()?;
        vfs.set_watch_enabled(self.watch);

        let session = ServeSession::new(vfs, project_path)?;
        let mut cursor = session.message_queue().cursor();

        write_model(&session, &output_path, output_kind)?;

        if self.watch {
            let rt = Runtime::new().context("Failed to start the async runtime for watch mode")?;

            loop {
                let receiver = session.message_queue().subscribe(cursor);
                let (new_cursor, _patch_set) = match rt.block_on(receiver) {
                    Ok(message) => message,
                    // The message queue was dropped, so there is nothing left
                    // to watch. Stop watching gracefully.
                    Err(_) => break,
                };
                cursor = new_cursor;

                write_model(&session, &output_path, output_kind)?;
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
) -> anyhow::Result<()> {
    println!("Building project '{}'", session.project_name());

    let tree = session.tree();
    let root_id = tree.get_root_id();

    log::trace!("Serializing to a staged output file");
    write_file_atomically(output, |file| {
        match output_kind {
            OutputKind::Rbxm => {
                rbx_binary::to_writer(file, tree.inner(), &[root_id])?;
            }
            OutputKind::Rbxl => {
                let root_instance = tree.get_instance(root_id).unwrap();
                let top_level_ids = root_instance.children();

                rbx_binary::to_writer(file, tree.inner(), top_level_ids)?;
            }
            OutputKind::Rbxmx => {
                // Model files include the root instance of the tree and all its
                // descendants.

                rbx_xml::to_writer(file, tree.inner(), &[root_id], xml_encode_config())?;
            }
            OutputKind::Rbxlx => {
                // Place files don't contain an entry for the DataModel, but our
                // WeakDom representation does.

                let root_instance = tree.get_instance(root_id).unwrap();
                let top_level_ids = root_instance.children();

                rbx_xml::to_writer(file, tree.inner(), top_level_ids, xml_encode_config())?;
            }
        }
        Ok(())
    })?;

    let filename = output
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("<invalid utf-8>");
    println!("Built project to {}", filename);

    Ok(())
}

/// Replaces `destination` with the bytes produced by `write`, leaving the
/// previous file untouched if writing fails.
///
/// The output is staged in a temporary file next to `destination` and renamed
/// over it once complete. Truncating the destination first would leave a
/// corrupt file behind whenever serialization fails, and Studio may load a
/// plugin or place file while it is half written.
///
/// If `destination` is a symlink, the file it points to is replaced instead,
/// so the link keeps working. Hard links to the previous file are not
/// updated, since the new contents are a different file.
fn write_file_atomically(
    destination: &Path,
    write: impl FnOnce(&mut dyn Write) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    // Renaming over a symlink would replace the link itself with a plain
    // file, and whatever it pointed to would silently stop being updated.
    let destination = &resolve_symlinks(destination)
        .with_context(|| format!("Could not resolve symlink {}", destination.display()))?;

    // The staged file must be on the same filesystem for the rename to be
    // atomic, so it goes in the destination's directory.
    let parent = match destination.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    #[cfg_attr(not(unix), allow(unused_mut))]
    let mut builder = tempfile::Builder::new();

    // Temporary files are created owner-only (0600) on Unix. A rebuild keeps
    // the permissions of the file it replaces, so it doesn't change who can
    // read it. A new file is created with 0666 like `File::create` does, so
    // that the user's umask applies to it.
    #[cfg(unix)]
    let previous_permissions = {
        use std::os::unix::fs::PermissionsExt;
        match std::fs::metadata(destination) {
            Ok(metadata) => Some(metadata.permissions()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                builder.permissions(std::fs::Permissions::from_mode(0o666));
                None
            }
            Err(err) => {
                return Err(err).with_context(|| {
                    format!(
                        "Could not read the permissions of {}",
                        destination.display()
                    )
                });
            }
        }
    };

    let mut staged = builder.tempfile_in(parent).with_context(|| {
        format!(
            "Could not create a temporary file in {} to write {}",
            parent.display(),
            destination.display()
        )
    })?;

    {
        let mut writer = BufWriter::new(staged.as_file_mut());
        write(&mut writer)?;
        writer.flush()?;
    }

    #[cfg(unix)]
    if let Some(permissions) = previous_permissions {
        staged.as_file().set_permissions(permissions)?;
    }

    // Without this, a crash shortly after the rename can leave an empty or
    // partial file in place of the output on filesystems that reorder the
    // rename before the data reaches the disk.
    staged.as_file().sync_all().with_context(|| {
        format!(
            "Could not flush the staged output for {}",
            destination.display()
        )
    })?;

    // Dropping the staged file on error deletes it, so a failed rename leaves
    // only the previous output behind.
    staged.persist(destination).map_err(|error| {
        anyhow!(
            "Could not replace {}: {}. The previous file was left unchanged; \
             if another program such as Roblox Studio has it open, close it and try again.",
            destination.display(),
            error.error
        )
    })?;

    // Persist the rename itself. This is best-effort: some filesystems do not
    // support syncing directories, and the output is already complete.
    #[cfg(unix)]
    if let Ok(directory) = std::fs::File::open(parent) {
        let _ = directory.sync_all();
    }

    Ok(())
}

/// Follows `path` through any chain of symlinks to the path that would be
/// opened for writing. Unlike `fs::canonicalize`, the final target does not
/// need to exist, and other components of the path are left as they are.
fn resolve_symlinks(path: &Path) -> std::io::Result<PathBuf> {
    // Matches the usual limit on symlink hops, so a loop is reported rather
    // than followed forever.
    const MAX_HOPS: usize = 40;

    let mut path = path.to_path_buf();
    for _ in 0..MAX_HOPS {
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                let target = std::fs::read_link(&path)?;
                path = match path.parent() {
                    Some(parent) if target.is_relative() => parent.join(target),
                    _ => target,
                };
            }
            _ => return Ok(path),
        }
    }

    Err(std::io::Error::other(format!(
        "too many levels of symbolic links at {}",
        path.display()
    )))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn failed_write_keeps_previous_file() {
        let dir = tempfile::tempdir().unwrap();
        let destination = dir.path().join("place.rbxl");
        std::fs::write(&destination, b"previous build").unwrap();

        let result = write_file_atomically(&destination, |file| {
            file.write_all(b"partial")?;
            bail!("serialization failed")
        });

        assert!(result.is_err());
        assert_eq!(std::fs::read(&destination).unwrap(), b"previous build");
        // The staged file is cleaned up rather than left next to the output.
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn successful_write_replaces_previous_file() {
        let dir = tempfile::tempdir().unwrap();
        let destination = dir.path().join("place.rbxl");
        std::fs::write(&destination, b"previous build").unwrap();

        write_file_atomically(&destination, |file| Ok(file.write_all(b"new build")?)).unwrap();

        assert_eq!(std::fs::read(&destination).unwrap(), b"new build");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    /// Creates a file symlink, or returns false where that needs privileges
    /// the test does not have (Windows without Developer Mode).
    fn symlink_file(target: &Path, link: &Path) -> bool {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target, link).unwrap();
            true
        }
        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_file(target, link).is_ok()
        }
    }

    #[test]
    fn write_through_symlink_replaces_link_target() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real.rbxl");
        let link = dir.path().join("link.rbxl");
        std::fs::write(&real, b"previous build").unwrap();
        if !symlink_file(Path::new("real.rbxl"), &link) {
            eprintln!("skipping: cannot create symlinks here");
            return;
        }

        write_file_atomically(&link, |file| Ok(file.write_all(b"new build")?)).unwrap();

        assert!(std::fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(std::fs::read(&real).unwrap(), b"new build");
        assert_eq!(std::fs::read(&link).unwrap(), b"new build");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
    }

    #[test]
    fn write_through_dangling_symlink_creates_link_target() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real.rbxl");
        let link = dir.path().join("link.rbxl");
        if !symlink_file(&real, &link) {
            eprintln!("skipping: cannot create symlinks here");
            return;
        }

        write_file_atomically(&link, |file| Ok(file.write_all(b"new build")?)).unwrap();

        assert!(std::fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(std::fs::read(&real).unwrap(), b"new build");
    }

    #[cfg(unix)]
    #[test]
    fn symlink_loop_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.rbxl");
        let b = dir.path().join("b.rbxl");
        std::os::unix::fs::symlink(&b, &a).unwrap();
        std::os::unix::fs::symlink(&a, &b).unwrap();

        assert!(write_file_atomically(&a, |file| Ok(file.write_all(b"new build")?)).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn new_file_respects_umask() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let destination = dir.path().join("place.rbxl");

        write_file_atomically(&destination, |file| Ok(file.write_all(b"new build")?)).unwrap();

        // A file created the way the output used to be, with File::create,
        // gets 0666 minus the current umask.
        let reference = dir.path().join("reference");
        std::fs::File::create(&reference).unwrap();
        let mode = |path: &Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&destination), mode(&reference));
    }

    #[cfg(unix)]
    #[test]
    fn rebuild_keeps_permissions_of_previous_file() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let destination = dir.path().join("place.rbxl");
        std::fs::write(&destination, b"previous build").unwrap();
        std::fs::set_permissions(&destination, std::fs::Permissions::from_mode(0o640)).unwrap();

        write_file_atomically(&destination, |file| Ok(file.write_all(b"new build")?)).unwrap();

        let mode = std::fs::metadata(&destination)
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o640);
    }
}
