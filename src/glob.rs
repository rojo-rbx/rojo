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
        let glob = <&str as Deserialize>::deserialize(deserializer)?;

        Glob::new(glob).map_err(D::Error::custom)
    }
}
