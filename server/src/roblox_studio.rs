//! Interactions with Roblox Studio's installation, including its location and
//! mechanisms like PluginSettings.

use std::path::PathBuf;
use std::env;

static ROJO_PLUGIN_ID: &'static str = "1211549683";

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