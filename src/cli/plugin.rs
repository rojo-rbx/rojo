use std::{
    fs::{self, File},
    io::BufWriter,
};

use clap::Parser;
use memofs::{InMemoryFs, Vfs, VfsSnapshot};
use roblox_install::RobloxStudio;

use crate::serve_session::ServeSession;

static PLUGIN_BINCODE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/plugin.bincode"));
static PLUGIN_FILE_NAME: &str = "RojoManagedPlugin.rbxm";

/// Install Rojo's plugin.
#[derive(Debug, Parser)]
pub struct PluginCommand {
    #[clap(subcommand)]
    subcommand: PluginSubcommand,
}

/// Manages Rojo's Roblox Studio plugin.
#[derive(Debug, Parser)]
pub enum PluginSubcommand {
    /// Install the plugin in Roblox Studio's plugins folder. If the plugin is
    /// already installed, installing it again will overwrite the current plugin
    /// file.
    Install,

    /// Removes the plugin if it is installed.
    Uninstall,
}

impl PluginCommand {
    pub fn run(self) -> anyhow::Result<()> {
        self.subcommand.run()
    }
}

impl PluginSubcommand {
    pub fn run(self) -> anyhow::Result<()> {
        match self {
            PluginSubcommand::Install => install_plugin(),
            PluginSubcommand::Uninstall => uninstall_plugin(),
        }
    }
}

fn install_plugin() -> anyhow::Result<()> {
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

    rbx_binary::to_writer(&mut file, tree.inner(), &[root_id])?;

    Ok(())
}

fn uninstall_plugin() -> anyhow::Result<()> {
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
