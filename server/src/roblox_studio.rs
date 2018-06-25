//! Interactions with Roblox Studio's installation, including its location and
//! mechanisms like PluginSettings.

#![allow(dead_code)]

use std::path::PathBuf;
use std::env;

#[cfg(all(not(debug_assertions), not(feature = "bundle-plugin")))]
compile_error!("`bundle-plugin` feature must be set for release builds.");

#[cfg(feature = "bundle-plugin")]
static PLUGIN_RBXM: &'static [u8] = include_bytes!("../target/plugin.rbxm");

const ROJO_HOTSWAP_PLUGIN_ID: &'static str = "0";
const ROJO_RELEASE_PLUGIN_ID: &'static str = "1997686364";

#[cfg(target_os = "windows")]
pub fn get_install_location() -> Option<PathBuf> {
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

#[cfg(feature = "bundle-plugin")]
pub fn get_plugin_location() -> Option<PathBuf> {
    let mut location = get_install_location()?;

    location.push("InstalledPlugins");
    location.push(ROJO_RELEASE_PLUGIN_ID);

    Some(location)
}

#[cfg(not(feature = "bundle-plugin"))]
pub fn get_plugin_location() -> Option<PathBuf> {
    let mut location = get_install_location()?;

    location.push("InstalledPlugins");
    location.push(ROJO_HOTSWAP_PLUGIN_ID);

    Some(location)
}

#[cfg(feature = "bundle-plugin")]
pub fn install_bundled_plugin() -> Option<()> {
    use std::fs::create_dir_all;

    println!("Installing plugin...");

    // TODO: Check error of this value; the only one we actually want to ignore
    // is ErrorKind::AlreadyExists probably.
    let _ = create_dir_all(get_plugin_location()?);

    // TODO: Copy PLUGIN_RBXM to plugin_location/Plugin.rbxm
    // TODO: Update PluginMetadata.json

    Some(())
}

#[cfg(not(feature = "bundle-plugin"))]
pub fn install_bundled_plugin() {
    println!("Skipping plugin installation, bundle-plugin not set.");
}