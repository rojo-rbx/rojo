use std::{
    io,
    fmt,
    path::{PathBuf},
};

use failure::Fail;

pub type FsResult<T> = Result<T, FsError>;
pub use io::ErrorKind as FsErrorKind;

/// A wrapper around io::Error that also attaches the path associated with the
/// error.
#[derive(Debug, Fail)]
pub struct FsError {
    #[fail(cause)]
    inner: io::Error,
    path: PathBuf,
}

impl FsError {
    pub fn new<P: Into<PathBuf>>(inner: io::Error, path: P) -> FsError {
        FsError {
            inner,
            path: path.into(),
        }
    }

    pub fn kind(&self) -> FsErrorKind {
        self.inner.kind()
    }
}

impl fmt::Display for FsError {
    fn fmt(&self, output: &mut fmt::Formatter) -> fmt::Result {
        write!(output, "{}: {}", self.path.display(), self.inner)
    }
}