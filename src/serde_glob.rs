//! Defines how to serialize and deserialize globs for Serde, since the globset
//! crate doesn't prescribe this.

use std::fmt;

use globset::Glob;
use serde::{
    de::{self, Visitor},
    Deserializer, Serializer,
};

pub fn serialize<S: Serializer>(glob: &Glob, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(glob.glob())
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

pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Glob, D::Error> {
    deserializer.deserialize_str(GlobVisitor)
}
