//! Path serializer is used to serialize absolute paths in a cross-platform way,
//! by replacing all directory separators with /.

use std::path::Path;

use serde::{ser::SerializeSeq, Serialize, Serializer};

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

pub fn serialize_option_absolute<S, T>(
    maybe_path: &Option<T>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: AsRef<Path>,
{
    match maybe_path {
        Some(path) => serialize_absolute(path, serializer),
        None => serializer.serialize_none(),
    }
}
