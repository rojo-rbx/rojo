/// A list of file names that are not valid on Windows.
const INVALID_WINDOWS_NAMES: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// A list of all characters that are outright forbidden to be included
/// in a file's name.
const FORBIDDEN_CHARS: [char; 9] = ['<', '>', ':', '"', '/', '|', '?', '*', '\\'];

/// Returns whether a given name is a valid file name. This takes into account
/// rules for Windows, MacOS, and Linux.
///
/// In practice however, these broadly overlap so the only unexpected behavior
/// is Windows, where there are 22 reserved names.
pub fn is_valid_file_name<S: AsRef<str>>(name: S) -> bool {
    let str = name.as_ref();

    if str.ends_with(' ') || str.ends_with('.') {
        return false;
    }

    for char in str.chars() {
        if char.is_control() || FORBIDDEN_CHARS.contains(&char) {
            return false;
        }
    }

    for forbidden in INVALID_WINDOWS_NAMES {
        if str == forbidden {
            return false;
        }
    }

    true
}
