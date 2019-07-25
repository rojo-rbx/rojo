use std::{
    io,
    fmt,
    path::{PathBuf},
};

use failure::Fail;

pub type FsResult<T> = Result<T, FsError>;
pub use io::ErrorKind as FsErrorKind;

// TODO: New error type that contains errors specific to our application,
// wrapping io::Error either directly or through another error type that has
// path information.
//
// It's possible that we should hoist up the path information one more level, or
// destructure/restructure information to hoist the path out of FsError and just
// embed io::Error?
pub enum ImfsError {
    NotFound,
    WrongKind,
    Fs(FsError),
}

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