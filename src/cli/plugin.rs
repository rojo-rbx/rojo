use std::{
    fs::{self, File},
    io::{self, BufWriter},
};

use anyhow::Result;
use memofs::{InMemoryFs, Vfs, VfsSnapshot};
use roblox_install::RobloxStudio;
use thiserror::Error;

use crate::{cli::{PluginCommand, PluginSubcommand}, serve_session::ServeSession};

static PLUGIN_FILE_NAME: &str = "RojoManagedPlugin.rbxmx";

#[derive(Debug, Error)]
enum Error {
    #[error("Could not locate Roblox Studio: {source}")]
    CannotLocateRobloxStudio { source: roblox_install::Error },

    #[error("{source}")]
    Io { source: io::Error },
}

pub fn plugin(options: PluginCommand) -> Result<()> {
    match options.subcommand {
        PluginSubcommand::Install => install_plugin(),
        PluginSubcommand::Uninstall => uninstall_plugin(),
    }
}

pub fn install_plugin() -> Result<()> {
    static PLUGIN_BINCODE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/plugin.bincode"));

    let plugin_snapshot: VfsSnapshot = bincode::deserialize(PLUGIN_BINCODE)
        .expect("Rojo's plugin was not properly packed into Rojo's binary");

    let studio = RobloxStudio::locate()
        .map_err(|source| Error::CannotLocateRobloxStudio { source })?;

    let plugins_folder_path = studio.plugins_path();

    if !plugins_folder_path.exists() {
        fs::create_dir(plugins_folder_path)
            .map_err(|source| Error::Io { source })?;
        log::trace!("Plugins folder did not exist so it was created");
    }

    let mut in_memory_fs = InMemoryFs::new();
    in_memory_fs
        .load_snapshot("plugin", plugin_snapshot)
        .map_err(|source| Error::Io { source })?;

    let vfs = Vfs::new(in_memory_fs);

    let session = ServeSession::new(vfs, "plugin");

    let tree = session.tree();

    log::trace!(
        "Writing plugin {} in {}",
        PLUGIN_FILE_NAME,
        plugins_folder_path.display()
    );
    let file = File::create(plugins_folder_path.join(PLUGIN_FILE_NAME))
        .map_err(|source| Error::Io { source })?;

    let mut file = BufWriter::new(file);

    let root_id = tree.get_root_id();

    rbx_binary::encode(tree.inner(), &[root_id], &mut file)
        .expect("Unable to encode Rojo's plugin");

    Ok(())
}

fn uninstall_plugin() -> Result<()> {
    let studio = RobloxStudio::locate()
        .map_err(|source| Error::CannotLocateRobloxStudio { source })?;

    let rojo_plugin_path = studio.plugins_path().join(PLUGIN_FILE_NAME);

    if rojo_plugin_path.exists() {
        log::trace!(
            "Removing existing plugin {}",
            rojo_plugin_path.display()
        );
        fs::remove_file(rojo_plugin_path)
            .map_err(|source| Error::Io { source })?;
    } else {
        log::trace!(
            "Plugin not installed {}",
            rojo_plugin_path.display()
        );
    }

    Ok(())
}
