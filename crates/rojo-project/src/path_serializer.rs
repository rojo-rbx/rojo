//! Path serializer is used to serialize absolute paths in a cross-platform way,
//! by replacing all directory separators with /.

use std::path::Path;

use serde::Serializer;

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
