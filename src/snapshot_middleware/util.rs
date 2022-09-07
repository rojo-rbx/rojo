use std::path::Path;

use anyhow::Context;

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

pub trait PathExt {
    fn file_name_ends_with(&self, suffix: &str) -> bool;
    fn file_name_trim_end<'a>(&'a self, suffix: &str) -> anyhow::Result<&'a str>;
    fn file_name_trim_extension<'a>(&'a self) -> anyhow::Result<String>;
}

impl<P> PathExt for P
where
    P: AsRef<Path>,
{
    fn file_name_ends_with(&self, suffix: &str) -> bool {
        self.as_ref()
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.ends_with(suffix))
            .unwrap_or(false)
    }

    fn file_name_trim_end<'a>(&'a self, suffix: &str) -> anyhow::Result<&'a str> {
        let path = self.as_ref();
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .with_context(|| format!("Path did not have a file name: {}", path.display()))?;

        match_trailing(&file_name, suffix)
            .with_context(|| format!("Path did not end in {}: {}", suffix, path.display()))
    }

    fn file_name_trim_extension(&self) -> anyhow::Result<String> {
        self.as_ref()
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(|string| string.to_owned())
            .with_context(|| format!("Path did not have a file name: {}", self.as_ref().display()))
    }
}
