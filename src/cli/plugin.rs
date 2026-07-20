use std::{
    fs::{self, File},
    io::{BufReader, BufWriter, Write},
    path::{Path, PathBuf},
};

use anyhow::Context;
use clap::Parser;
use memofs::{InMemoryFs, Vfs, VfsSnapshot};
use rbx_dom_weak::types::Variant;
use roblox_install::RobloxStudio;

use crate::serve_session::ServeSession;

static PLUGIN_BINCODE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/plugin.bincode"));
const PLUGIN_FILE_NAME: &str = "PrismManagedPlugin.rbxm";
const LEGACY_PLUGIN_FILE_NAME: &str = "RojoManagedPlugin.rbxm";
const PRISM_ASSET_ID: &str = "rbxassetid://84145747248222";

/// Manage Prism's local Roblox Studio plugin.
#[derive(Debug, Parser)]
pub struct PluginCommand {
    #[clap(subcommand)]
    subcommand: PluginSubcommand,
}

/// Manage Prism's local Roblox Studio plugin.
#[derive(Debug, Parser)]
pub enum PluginSubcommand {
    /// Install or update Prism's managed plugin in Roblox Studio's Plugins
    /// folder.
    Install,

    /// List local Prism and Rojo-like plugin files without changing them.
    List,

    /// Remove Prism's managed plugin if it is installed.
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
            PluginSubcommand::List => list_plugins(),
            PluginSubcommand::Uninstall => uninstall_plugin(),
        }
    }
}

fn initialize_plugin() -> anyhow::Result<ServeSession> {
    let plugin_snapshot: VfsSnapshot = bincode::deserialize(PLUGIN_BINCODE)
        .expect("Prism's plugin was not properly packed into the Prism binary");

    let mut in_memory_fs = InMemoryFs::new();
    in_memory_fs.load_snapshot("/plugin", plugin_snapshot)?;

    let vfs = Vfs::new(in_memory_fs);
    Ok(ServeSession::new(vfs, "/plugin")?)
}

fn managed_plugin_path(plugins_folder: &Path) -> PathBuf {
    plugins_folder.join(PLUGIN_FILE_NAME)
}

fn install_plugin() -> anyhow::Result<()> {
    let studio = RobloxStudio::locate()?;
    let plugins_folder = studio.plugins_path();

    if !plugins_folder.exists() {
        log::debug!("Creating Roblox Studio plugins folder");
        fs::create_dir_all(plugins_folder)?;
    }

    for plugin in inspect_local_plugins(plugins_folder)? {
        match plugin.kind {
            LocalPluginKind::LegacyPrismManaged => {
                log::info!(
                    "Removing known Prism-owned legacy plugin {}",
                    plugin.path.display()
                );
                fs::remove_file(&plugin.path).with_context(|| {
                    format!(
                        "Could not remove known Prism-owned legacy plugin '{}'.",
                        plugin.path.display()
                    )
                })?;
            }
            LocalPluginKind::RojoManagedUnknown => log::warn!(
                "Found {}. Prism cannot reliably determine its owner, so it was not changed. Remove or disable it manually if it is this fork's old managed plugin.",
                plugin.path.display()
            ),
            LocalPluginKind::PrismManaged
            | LocalPluginKind::PrismLocalBuild
            | LocalPluginKind::PossiblePrismLocalBuild => {
                if plugin.path.file_name().is_some_and(|name| name != PLUGIN_FILE_NAME) {
                    log::warn!(
                        "Found another local Prism plugin at {}. Disable or remove duplicates manually if Studio loads both.",
                        plugin.path.display()
                    );
                }
            }
            LocalPluginKind::RojoLike => {}
        }
    }

    let plugin_path = managed_plugin_path(plugins_folder);
    log::debug!("Writing plugin to {}", plugin_path.display());

    let mut file = BufWriter::new(File::create(&plugin_path)?);
    let session = initialize_plugin()?;
    let tree = session.tree();
    let root_id = tree.get_root_id();
    rbx_binary::to_writer(&mut file, tree.inner(), &[root_id])?;
    file.flush()?;

    println!("Installed Prism Studio plugin at {}", plugin_path.display());
    println!(
        "Marketplace-installed plugins are managed separately in Studio; disable any duplicate Rojo or Prism plugin there."
    );
    Ok(())
}

fn list_plugins() -> anyhow::Result<()> {
    let studio = RobloxStudio::locate()?;
    let plugins_folder = studio.plugins_path();
    let plugins = inspect_local_plugins(plugins_folder)?;

    println!("{}", render_local_plugin_report(plugins_folder, &plugins));
    Ok(())
}

fn render_local_plugin_report(plugins_folder: &Path, plugins: &[LocalPlugin]) -> String {
    let mut lines = vec![format!(
        "Local Roblox plugins in {}:",
        plugins_folder.display()
    )];
    if plugins.is_empty() {
        lines.push("  No known Prism or Rojo-like local plugin files found.".to_owned());
    } else {
        for plugin in plugins {
            lines.push(format!(
                "  [{}] {}",
                plugin.kind.label(),
                plugin.path.display()
            ));
        }

        let prism_count = plugins
            .iter()
            .filter(|plugin| plugin.kind.is_prism())
            .count();
        if prism_count > 1 {
            lines.push(format!(
                "  Warning: {prism_count} likely Prism local plugins were found; Studio may load duplicates."
            ));
        }
    }
    lines.push(
        "Marketplace-installed plugins do not necessarily appear here; disable duplicate Rojo or Prism plugins through Studio's plugin manager."
            .to_owned(),
    );
    lines.join("\n")
}

fn uninstall_plugin() -> anyhow::Result<()> {
    let studio = RobloxStudio::locate()?;
    let plugin_path = managed_plugin_path(studio.plugins_path());

    if plugin_path.exists() {
        fs::remove_file(&plugin_path)?;
        println!("Removed Prism Studio plugin at {}", plugin_path.display());
    } else {
        println!(
            "Prism Studio plugin is not installed at {}",
            plugin_path.display()
        );
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LocalPluginKind {
    PrismManaged,
    LegacyPrismManaged,
    PrismLocalBuild,
    PossiblePrismLocalBuild,
    RojoManagedUnknown,
    RojoLike,
}

impl LocalPluginKind {
    fn label(self) -> &'static str {
        match self {
            Self::PrismManaged => "Prism managed plugin",
            Self::LegacyPrismManaged => "legacy Prism-managed plugin (old Rojo filename)",
            Self::PrismLocalBuild => "known Prism local build",
            Self::PossiblePrismLocalBuild => "possible Prism local build",
            Self::RojoManagedUnknown => "Rojo managed plugin (ownership ambiguous)",
            Self::RojoLike => "Rojo-like local plugin (not managed by Prism)",
        }
    }

    fn is_prism(self) -> bool {
        matches!(
            self,
            Self::PrismManaged
                | Self::LegacyPrismManaged
                | Self::PrismLocalBuild
                | Self::PossiblePrismLocalBuild
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LocalPlugin {
    path: PathBuf,
    kind: LocalPluginKind,
}

fn inspect_local_plugins(plugins_folder: &Path) -> anyhow::Result<Vec<LocalPlugin>> {
    if !plugins_folder.exists() {
        return Ok(Vec::new());
    }

    let mut plugins = Vec::new();
    for entry in fs::read_dir(plugins_folder).with_context(|| {
        format!(
            "Could not inspect Roblox Studio's Plugins folder '{}'.",
            plugins_folder.display()
        )
    })? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }

        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let lower_name = file_name.to_ascii_lowercase();
        let kind = if file_name.eq_ignore_ascii_case(PLUGIN_FILE_NAME) {
            Some(LocalPluginKind::PrismManaged)
        } else if file_name.eq_ignore_ascii_case(LEGACY_PLUGIN_FILE_NAME) {
            Some(if is_prism_owned_legacy_plugin(&path).unwrap_or(false) {
                LocalPluginKind::LegacyPrismManaged
            } else {
                LocalPluginKind::RojoManagedUnknown
            })
        } else if file_name.eq_ignore_ascii_case("PrismPlugin.rbxm") {
            Some(LocalPluginKind::PrismLocalBuild)
        } else if ["rojoexecplugin.rbxm", "rojoexecspike.rbxm"].contains(&lower_name.as_str()) {
            Some(LocalPluginKind::RojoLike)
        } else if (lower_name.ends_with(".rbxm") || lower_name.ends_with(".rbxmx"))
            && lower_name.contains("prism")
        {
            Some(LocalPluginKind::PossiblePrismLocalBuild)
        } else {
            None
        };

        if let Some(kind) = kind {
            plugins.push(LocalPlugin { path, kind });
        }
    }
    plugins.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(plugins)
}

fn is_prism_owned_legacy_plugin(path: &Path) -> anyhow::Result<bool> {
    let file = BufReader::new(
        File::open(path)
            .with_context(|| format!("Could not open legacy plugin '{}'.", path.display()))?,
    );
    let dom = rbx_binary::from_reader(file)
        .with_context(|| format!("Could not inspect legacy plugin '{}'.", path.display()))?;
    let mut has_prism_asset = false;
    let mut has_prism_identity = false;

    for instance in dom.descendants() {
        for value in instance.properties.values() {
            if let Variant::String(value) = value {
                has_prism_asset |= value.contains(PRISM_ASSET_ID);
                has_prism_identity |= value.contains("local pluginName = \"Prism ")
                    || value.contains("\"name\": \"Prism\"");
            }
        }
    }

    Ok(has_prism_asset && has_prism_identity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rbx_dom_weak::{InstanceBuilder, WeakDom};

    #[test]
    fn plugin_initialize_has_no_exec_spike_modules() {
        let session = initialize_plugin().unwrap();
        let tree = session.tree();
        let names = tree
            .descendants(tree.get_root_id())
            .map(|instance| instance.name().to_owned())
            .collect::<Vec<_>>();

        assert!(!names.iter().any(|name| name.contains("ExecSpike")));
    }

    #[test]
    fn managed_plugin_output_path_is_prism_branded() {
        let path = managed_plugin_path(Path::new(
            r"C:\Users\Developer\AppData\Local\Roblox\Plugins",
        ));
        assert_eq!(path.file_name().unwrap(), PLUGIN_FILE_NAME);
        assert_eq!(PLUGIN_FILE_NAME, "PrismManagedPlugin.rbxm");
    }

    #[test]
    fn diagnostics_are_deterministic_and_distinguish_known_names() {
        let directory = tempfile::tempdir().unwrap();
        for name in [
            "z-prism-copy.rbxm",
            "RojoExecSpike.rbxm",
            "PrismPlugin.rbxm",
            "PrismManagedPlugin.rbxm",
            "unrelated.rbxm",
        ] {
            File::create(directory.path().join(name)).unwrap();
        }

        let plugins = inspect_local_plugins(directory.path()).unwrap();
        let names = plugins
            .iter()
            .map(|plugin| {
                plugin
                    .path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            [
                "PrismManagedPlugin.rbxm",
                "PrismPlugin.rbxm",
                "RojoExecSpike.rbxm",
                "z-prism-copy.rbxm",
            ]
        );
        assert_eq!(plugins[0].kind, LocalPluginKind::PrismManaged);
        assert_eq!(plugins[1].kind, LocalPluginKind::PrismLocalBuild);
        assert_eq!(plugins[2].kind, LocalPluginKind::RojoLike);
        assert_eq!(plugins[3].kind, LocalPluginKind::PossiblePrismLocalBuild);

        let report = render_local_plugin_report(directory.path(), &plugins);
        assert!(report.contains("[Prism managed plugin]"));
        assert!(report.contains("[Rojo-like local plugin (not managed by Prism)]"));
        assert!(report.contains("Warning: 3 likely Prism local plugins"));
        assert!(report.contains("Marketplace-installed plugins"));
    }

    #[test]
    fn recognizes_only_signed_legacy_prism_plugins() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(LEGACY_PLUGIN_FILE_NAME);
        let dom = WeakDom::new(
            InstanceBuilder::new("Folder")
                .with_child(
                    InstanceBuilder::new("ModuleScript")
                        .with_name("Assets")
                        .with_property("Source", format!("Logo = {PRISM_ASSET_ID}")),
                )
                .with_child(
                    InstanceBuilder::new("ModuleScript")
                        .with_name("App")
                        .with_property("Source", "local pluginName = \"Prism \""),
                ),
        );
        let mut file = BufWriter::new(File::create(&path).unwrap());
        rbx_binary::to_writer(&mut file, &dom, dom.root().children()).unwrap();
        drop(file);

        assert!(is_prism_owned_legacy_plugin(&path).unwrap());

        let unsigned_path = directory.path().join("unsigned.rbxm");
        let unsigned = WeakDom::new(InstanceBuilder::new("Folder"));
        let mut file = BufWriter::new(File::create(&unsigned_path).unwrap());
        rbx_binary::to_writer(&mut file, &unsigned, unsigned.root().children()).unwrap();
        drop(file);
        assert!(!is_prism_owned_legacy_plugin(&unsigned_path).unwrap());
    }
}
