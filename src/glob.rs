//! Wrapper around globset's Glob type that has better serialization
//! characteristics by coupling Glob and GlobMatcher into a single type.

use std::{fmt, path::Path};

use globset::{Glob as InnerGlob, GlobMatcher};
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};

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
        deserializer.deserialize_str(GlobVisitor)
    }
}

struct GlobVisitor;

impl<'de> Visitor<'de> for GlobVisitor {
    type Value = Glob;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string containing a glob pattern")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Glob::new(value).map_err(E::custom)
    }
}
