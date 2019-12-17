use std::{error::Error, fmt, io, path::PathBuf};

pub type FsResult<T> = Result<T, FsError>;
pub use io::ErrorKind as FsErrorKind;

pub trait FsResultExt<T> {
    fn with_not_found(self) -> Result<Option<T>, FsError>;
}

impl<T> FsResultExt<T> for Result<T, FsError> {
    fn with_not_found(self) -> Result<Option<T>, FsError> {
        match self {
            Ok(value) => Ok(Some(value)),
            Err(ref err) if err.kind() == FsErrorKind::NotFound => Ok(None),
            Err(err) => Err(err),
        }
    }
}

/// A wrapper around io::Error that also attaches the path associated with the
/// error.
#[derive(Debug)]
pub struct FsError {
    source: io::Error,
    path: PathBuf,
}

impl FsError {
    pub fn new<P: Into<PathBuf>>(source: io::Error, path: P) -> FsError {
        FsError {
            source,
            path: path.into(),
        }
    }

    pub fn kind(&self) -> FsErrorKind {
        self.source.kind()
    }

    pub fn into_raw(self) -> (io::Error, PathBuf) {
        (self.source, self.path)
    }
}

impl Error for FsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

impl fmt::Display for FsError {
    fn fmt(&self, output: &mut fmt::Formatter) -> fmt::Result {
        write!(output, "{}: {}", self.path.display(), self.source)
    }
}
