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
            let value = match pieces.next() {
                Some(value) => value,
                None => {
                    log::warn!("Unrecognized Roblox Studio cookie value: missing COOK value");
                    return None;
                }
            };

            if !value.starts_with('<') || !value.ends_with('>') {
                log::warn!("Unrecognized Roblox Studio cookie value: was not wrapped in <>");
                return None;
            }

            let value = &value[1..value.len() - 1];

            cookie = Some(value);
        }
    }

    cookie.map(Into::into)
}

#[cfg(not(windows))]
pub fn get_auth_cookie() -> Option<String> {
    None
}
