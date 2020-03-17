use crate::serve_session::ServeSession;

use memofs::InMemoryFs;
use memofs::{Vfs, VfsSnapshot};
use roblox_install::RobloxStudio;
use snafu::{ResultExt, Snafu};
use std::{
    fs::{self, File},
    io::{self, BufWriter},
};
use structopt::StructOpt;

static PLUGIN_FILE_NAME: &str = concat!("rojo-", env!("CARGO_PKG_VERSION"), ".rbxmx");

#[derive(Debug, StructOpt)]
enum PluginSubcommand {
    /// Install the plugin in Roblox plugins folder. If the plugin is already installed, installing
    /// it again will overwrite the current plugin file.
    Install,
    /// Removes the plugin if installed.
    Uninstall,
}

/// Install rojo's plugin.
#[derive(Debug, StructOpt)]
pub struct PluginCommand {
    #[structopt(subcommand)]
    subcommand: PluginSubcommand,
}

#[derive(Debug, Snafu)]
pub struct PluginError(Error);

#[derive(Debug, Snafu)]
enum Error {
    #[snafu(display("Could not locate Roblox Studio: {}", source.to_string()))]
    CannotLocateRobloxStudio { source: roblox_install::Error },

    #[snafu(display("{}", source))]
    Io { source: io::Error },
}

fn xml_encode_config() -> rbx_xml::EncodeOptions {
    rbx_xml::EncodeOptions::new().property_behavior(rbx_xml::EncodePropertyBehavior::WriteUnknown)
}

pub fn plugin(options: PluginCommand) -> Result<(), PluginError> {
    match options.subcommand {
        PluginSubcommand::Install => install_plugin()?,
        PluginSubcommand::Uninstall => uninstall_plugin()?,
    }
    Ok(())
}

pub fn install_plugin() -> Result<(), PluginError> {
    static PLUGIN_BINCODE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/plugin.bincode"));

    let plugin_snapshot: VfsSnapshot = bincode::deserialize(PLUGIN_BINCODE)
        .expect("rojo's plugin was not properly packed into rojo's binary");

    let studio = RobloxStudio::locate().context(CannotLocateRobloxStudio)?;

    let plugins_folder_path = studio.plugins_path();

    if !plugins_folder_path.exists() {
        fs::create_dir(plugins_folder_path).context(Io)?;
        log::trace!("Plugins folder did not exist so it was created");
    }

    let mut in_memory_fs = InMemoryFs::new();
    in_memory_fs
        .load_snapshot("plugin", plugin_snapshot)
        .context(Io)?;

    let vfs = Vfs::new(in_memory_fs);

    let session = ServeSession::new(vfs, "plugin");

    let tree = session.tree();

    log::trace!(
        "Writing plugin {} in {}",
        PLUGIN_FILE_NAME,
        plugins_folder_path.to_string_lossy()
    );
    let file = File::create(plugins_folder_path.join(PLUGIN_FILE_NAME)).context(Io)?;

    let mut file = BufWriter::new(file);

    let root_id = tree.get_root_id();

    rbx_xml::to_writer(&mut file, tree.inner(), &[root_id], xml_encode_config()).unwrap();

    Ok(())
}

fn uninstall_plugin() -> Result<(), PluginError> {
    let studio = RobloxStudio::locate().context(CannotLocateRobloxStudio)?;

    let rojo_plugin_path = studio.plugins_path().join(PLUGIN_FILE_NAME);

    if rojo_plugin_path.exists() {
        log::trace!(
            "Removing existing plugin {}",
            rojo_plugin_path.to_string_lossy()
        );
        fs::remove_file(rojo_plugin_path).context(Io)?;
    } else {
        log::trace!(
            "Plugin not installed {}",
            rojo_plugin_path.to_string_lossy()
        );
    }

    Ok(())
}
