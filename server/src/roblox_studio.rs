//! Interactions with Roblox Studio's installation, including its location and
//! mechanisms like PluginSettings.

#![allow(dead_code)]

use std::path::PathBuf;

#[cfg(all(not(debug_assertions), not(feature = "bundle-plugin")))]
compile_error!("`bundle-plugin` feature must be set for release builds.");

#[cfg(feature = "bundle-plugin")]
static PLUGIN_RBXM: &'static [u8] = include_bytes!("../target/plugin.rbxmx");

#[cfg(target_os = "windows")]
pub fn get_install_location() -> Option<PathBuf> {
    use std::env;

    let local_app_data = env::var("LocalAppData").ok()?;
    let mut location = PathBuf::from(local_app_data);

    location.push("Roblox");

    Some(location)
}

#[cfg(target_os = "macos")]
pub fn get_install_location() -> Option<PathBuf> {
    unimplemented!();
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn get_install_location() -> Option<PathBuf> {
    // Roblox Studio doesn't install on any other platforms!
    None
}

pub fn get_plugin_location() -> Option<PathBuf> {
    let mut location = get_install_location()?;

    location.push("Plugins/Rojo.rbxmx");

    Some(location)
}

#[cfg(feature = "bundle-plugin")]
pub fn install_bundled_plugin() -> Option<()> {
    use std::fs::File;
    use std::io::Write;

    info!("Installing plugin...");

    let mut file = File::create(get_plugin_location()?).ok()?;
    file.write_all(PLUGIN_RBXM).ok()?;

    Some(())
}

#[cfg(not(feature = "bundle-plugin"))]
pub fn install_bundled_plugin() -> Option<()> {
    info!("Skipping plugin installation, bundle-plugin not set.");

    Some(())
}