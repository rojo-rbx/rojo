//! Implementation of automatically fetching authentication cookie from a Roblox
//! Studio installation.

#[cfg(windows)]
pub fn get_auth_cookie() -> Option<String> {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let cookies = hkcu
        .open_subkey("Software\\Roblox\\RobloxStudioBrowser\\roblox.com")
        .ok()?;

    let entry: String = cookies.get_value(".ROBLOSECURITY").ok()?;
    let mut cookie = None;

    for kv_pair in entry.split(",") {
        let mut pieces = kv_pair.split("::");

        if let Some("COOK") = pieces.next() {
            cookie = pieces.next();
        }
    }

    cookie.map(Into::into)
}

#[cfg(not(windows))]
pub fn get_auth_cookie() -> Option<String> {
    None
}
