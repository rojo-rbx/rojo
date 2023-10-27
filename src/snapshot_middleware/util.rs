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
    fn file_name_trim_extensions(&self) -> anyhow::Result<&str>;
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

        match_trailing(file_name, suffix)
            .with_context(|| format!("Path did not end in {}: {}", suffix, path.display()))
    }

    /// Returns the name of a file after all extensions have been removed.
    fn file_name_trim_extensions(&self) -> anyhow::Result<&str> {
        // I would love for this to be less verbose, but I'm not sure it's
        // really possible this. It doesn't allocate, so it's no huge loss
        // either way though.
        let mut file_name = self
            .as_ref()
            .file_stem()
            .and_then(|n| n.to_str())
            .with_context(|| format!("file name of {} is invalid", self.as_ref().display()))?;
        while Path::new(file_name).extension().is_some() {
            file_name = Path::new(file_name)
                .file_stem()
                .and_then(|n| n.to_str())
                .with_context(|| format!("file name of {} is invalid", self.as_ref().display()))?;
        }
        Ok(file_name)
    }
}

// TEMP function until rojo 8.0, when it can be replaced with bool::default (aka false)
pub fn emit_legacy_scripts_default() -> Option<bool> {
    Some(true)
}

#[test]
fn file_name_trim_extensions_invalid() {
    // Basic test to make sure that the loop for this function
    // isn't infinite and that it works right
    assert!(Path::new("").file_name_trim_extensions().is_err());

    assert_eq!(Path::new("foo").file_name_trim_extensions().unwrap(), "foo");
    assert_eq!(
        Path::new("foo.bar").file_name_trim_extensions().unwrap(),
        "foo"
    );
    assert_eq!(
        Path::new("foo.bar.baz")
            .file_name_trim_extensions()
            .unwrap(),
        "foo"
    );
}
