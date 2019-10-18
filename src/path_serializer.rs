//! path_serializer is used in cases where we need to serialize relative Path
//! and PathBuf objects in a way that's cross-platform.
//!
//! This is used for the snapshot testing system to make sure that snapshots
//! that reference local paths that are generated on Windows don't fail when run
//! in systems that use a different directory separator.
//!
//! To use, annotate your PathBuf or Option<PathBuf> field with the correct
//! serializer function:
//!
//! ```ignore
//! # use std::path::PathBuf;
//! # use serde::{Serialize, Deserialize};
//!
//! #[derive(Serialize, Deserialize)]
//! struct Mine {
//!     name: String,
//!
//!     // Use 'crate' instead of librojo if writing code inside Rojo
//!     #[serde(serialize_with = "librojo::path_serializer::serialize")]
//!     source_path: PathBuf,
//!
//!     #[serde(serialize_with = "librojo::path_serializer::serialize_option")]
//!     maybe_path: Option<PathBuf>,
//! }
//! ```
//!
//! For absolute paths, which are only safe to serialize if they're artificial,
//! use `serialize_absolute`.

use std::path::{Component, Path};

use serde::{ser::SerializeSeq, Serialize, Serializer};

pub fn serialize_option<S, T>(maybe_path: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: AsRef<Path>,
{
    match maybe_path {
        Some(path) => serialize(path, serializer),
        None => serializer.serialize_none(),
    }
}

pub fn serialize<S, T>(path: T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: AsRef<Path>,
{
    let path = path.as_ref();

    assert!(
        path.is_relative(),
        "path_serializer can only handle relative paths"
    );

    let mut output = String::new();

    for component in path.components() {
        if !output.is_empty() {
            output.push('/');
        }

        match component {
            Component::CurDir => output.push('.'),
            Component::ParentDir => output.push_str(".."),
            Component::Normal(piece) => output.push_str(piece.to_str().unwrap()),
            _ => panic!("path_serializer cannot handle absolute path components"),
        }
    }

    serializer.serialize_str(&output)
}

pub fn serialize_absolute<S, T>(path: T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: AsRef<Path>,
{
    let as_str = path
        .as_ref()
        .as_os_str()
        .to_str()
        .expect("Invalid Unicode in file path, cannot serialize");
    let replaced = as_str.replace("\\", "/");

    serializer.serialize_str(&replaced)
}

#[derive(Serialize)]
struct WithAbsolute<'a>(#[serde(serialize_with = "serialize_absolute")] &'a Path);

pub fn serialize_vec_absolute<S, T>(paths: &[T], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: AsRef<Path>,
{
    let mut seq = serializer.serialize_seq(Some(paths.len()))?;

    for path in paths {
        seq.serialize_element(&WithAbsolute(path.as_ref()))?;
    }

    seq.end()
}
