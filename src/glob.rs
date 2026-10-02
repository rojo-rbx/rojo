//! Wrapper around globset's Glob type that has better serialization
//! characteristics by coupling Glob and GlobMatcher into a single type.

use std::path::Path;

use globset::{Glob as InnerGlob, GlobMatcher};
use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};

pub use globset::Error;

#[derive(Debug, Clone)]
pub struct Glob {
    inner: InnerGlob,
    matcher: GlobMatcher,
}

impl Glob {
    pub fn new(glob: &str) -> Result<Self, Error> {
        let inner = InnerGlob::new(glob)?;
        let matcher = inner.compile_matcher();

        Ok(Glob { inner, matcher })
    }

    pub fn is_match<P: AsRef<Path>>(&self, path: P) -> bool {
        self.matcher.is_match(path)
    }
}

impl PartialEq for Glob {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl Eq for Glob {}

impl Serialize for Glob {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.inner.glob())
    }
}

impl<'de> Deserialize<'de> for Glob {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let glob = String::deserialize(deserializer)?;

        Glob::new(&glob).map_err(D::Error::custom)
    }
}

/// A glob with optional gitignore-style negation. A leading `!` marks the
/// pattern as a negation (re-includes paths that an earlier rule excluded).
/// To match a literal `!` at the start of a pattern, escape it with `\!`.
#[derive(Debug, Clone)]
pub struct IgnorableGlob {
    glob: Glob,
    negated: bool,
    raw: String,
}

impl IgnorableGlob {
    pub fn new(pattern: &str) -> Result<Self, Error> {
        let (negated, body) = if let Some(rest) = pattern.strip_prefix('!') {
            (true, rest)
        } else if pattern.starts_with(r"\!") {
            (false, &pattern[1..])
        } else {
            (false, pattern)
        };

        Ok(IgnorableGlob {
            glob: Glob::new(body)?,
            negated,
            raw: pattern.to_owned(),
        })
    }

    pub fn is_match<P: AsRef<Path>>(&self, path: P) -> bool {
        self.glob.is_match(path)
    }

    pub fn is_negation(&self) -> bool {
        self.negated
    }
}

impl PartialEq for IgnorableGlob {
    fn eq(&self, other: &Self) -> bool {
        self.negated == other.negated && self.glob == other.glob
    }
}

impl Eq for IgnorableGlob {}

impl Serialize for IgnorableGlob {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.raw)
    }
}

impl<'de> Deserialize<'de> for IgnorableGlob {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let pattern = String::deserialize(deserializer)?;

        IgnorableGlob::new(&pattern).map_err(D::Error::custom)
    }
}
