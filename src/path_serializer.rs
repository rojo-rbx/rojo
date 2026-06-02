//! Path serializer is used to serialize absolute paths in a cross-platform way,
//! by replacing all directory separators with /.

use std::path::Path;

use serde::{ser::SerializeSeq, Serialize, Serializer};

/// Converts the provided value into a String with all directory separators
/// converted into `/`.
///
/// Paths that contain invalid Unicode are converted lossily (invalid sequences
/// become the replacement character), since such paths cannot be represented
/// faithfully in a UTF-8 string regardless.
pub fn display_absolute<T: AsRef<Path>>(path: T) -> String {
    path.as_ref().to_string_lossy().replace('\\', "/")
}

/// A serializer for serde that serialize a value with all directory separators
/// converted into `/`.
pub fn serialize_absolute<S, T>(path: T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: AsRef<Path>,
{
    serializer.serialize_str(&display_absolute(path))
}

#[derive(Serialize)]
struct WithAbsolute<'a>(#[serde(serialize_with = "serialize_absolute")] &'a Path);

/// A serializer for serde that serialize a list of values with all directory
/// separators converted into `/`.
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
