use std::path::Path;

/// If the given string ends up with the given suffix, returns the portion of
/// the string before the suffix.
pub fn match_trailing<'a>(input: &'a str, suffix: &str) -> Option<&'a str> {
    if input.ends_with(suffix) {
        let end = input.len().saturating_sub(suffix.len());
        Some(&input[..end])
    } else {
        None
    }
}

/// If the given path has a file name, and that file name ends with the given
/// suffix, returns the portion of the file name before the given suffix.
pub fn match_file_name<'a>(path: &'a Path, suffix: &str) -> Option<&'a str> {
    let file_name = path.file_name()?.to_str()?;

    match_trailing(&file_name, suffix)
}
