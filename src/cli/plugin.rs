use std::{
    fs::{self, File},
    io::BufWriter,
};

use anyhow::Result;
use memofs::{InMemoryFs, Vfs, VfsSnapshot};
use roblox_install::RobloxStudio;

use crate::{
    cli::{PluginCommand, PluginSubcommand},
    serve_session::ServeSession,
};

static PLUGIN_BINCODE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/plugin.bincode"));
static PLUGIN_FILE_NAME: &str = "RojoManagedPlugin.rbxm";

pub fn plugin(options: PluginCommand) -> Result<()> {
    match options.subcommand {
        PluginSubcommand::Install => install_plugin(),
        PluginSubcommand::Uninstall => uninstall_plugin(),
    }
}

pub fn install_plugin() -> Result<()> {
    let plugin_snapshot: VfsSnapshot = bincode::deserialize(PLUGIN_BINCODE)
        .expect("Rojo's plugin was not properly packed into Rojo's binary");

    let studio = RobloxStudio::locate()?;

    let plugins_folder_path = studio.plugins_path();

    if !plugins_folder_path.exists() {
        log::debug!("Creating Roblox Studio plugins folder");
        fs::create_dir(plugins_folder_path)?;
    }

    let mut in_memory_fs = InMemoryFs::new();
    in_memory_fs.load_snapshot("/plugin", plugin_snapshot)?;

    let vfs = Vfs::new(in_memory_fs);
    let session = ServeSession::new(vfs, "/plugin")?;

    let plugin_path = plugins_folder_path.join(PLUGIN_FILE_NAME);
    log::debug!("Writing plugin to {}", plugin_path.display());

    let mut file = BufWriter::new(File::create(plugin_path)?);

    let tree = session.tree();
    let root_id = tree.get_root_id();

    rbx_binary::encode(tree.inner(), &[root_id], &mut file)?;

    Ok(())
}

fn uninstall_plugin() -> Result<()> {
    let studio = RobloxStudio::locate()?;

    let plugin_path = studio.plugins_path().join(PLUGIN_FILE_NAME);

    if plugin_path.exists() {
        log::debug!("Removing existing plugin from {}", plugin_path.display());
        fs::remove_file(plugin_path)?;
    } else {
        log::debug!("Plugin not installed at {}", plugin_path.display());
    }

    Ok(())
}
