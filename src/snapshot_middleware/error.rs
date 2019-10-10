use std::{error::Error, fmt, io, path::PathBuf};

use crate::imfs::FsError;

#[derive(Debug)]
pub struct SnapshotError {
    detail: SnapshotErrorDetail,
    path: Option<PathBuf>,
}

impl SnapshotError {
    pub fn new(detail: SnapshotErrorDetail, path: Option<impl Into<PathBuf>>) -> Self {
        SnapshotError {
            detail,
            path: path.map(Into::into),
        }
    }

    pub(crate) fn wrap(inner: impl Into<SnapshotErrorDetail>, path: impl Into<PathBuf>) -> Self {
        SnapshotError {
            detail: inner.into(),
            path: Some(path.into()),
        }
    }

    pub(crate) fn file_did_not_exist(path: impl Into<PathBuf>) -> SnapshotError {
        SnapshotError {
            detail: SnapshotErrorDetail::FileDidNotExist,
            path: Some(path.into()),
        }
    }

    pub(crate) fn file_name_bad_unicode(path: impl Into<PathBuf>) -> SnapshotError {
        SnapshotError {
            detail: SnapshotErrorDetail::FileNameBadUnicode,
            path: Some(path.into()),
        }
    }

    pub(crate) fn file_contents_bad_unicode(
        inner: std::str::Utf8Error,
        path: impl Into<PathBuf>,
    ) -> SnapshotError {
        SnapshotError {
            detail: SnapshotErrorDetail::FileContentsBadUnicode { inner },
            path: Some(path.into()),
        }
    }

    pub(crate) fn malformed_project(
        inner: serde_json::Error,
        path: impl Into<PathBuf>,
    ) -> SnapshotError {
        SnapshotError {
            detail: SnapshotErrorDetail::MalformedProject { inner },
            path: Some(path.into()),
        }
    }
}

impl Error for SnapshotError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.detail.source()
    }
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match &self.path {
            Some(path) => write!(formatter, "{} in path {}", self.detail, path.display()),
            None => write!(formatter, "{}", self.detail),
        }
    }
}

impl From<FsError> for SnapshotError {
    fn from(error: FsError) -> Self {
        let (inner, path) = error.into_raw();

        Self::new(inner.into(), Some(path))
    }
}

impl From<rlua::Error> for SnapshotError {
    fn from(error: rlua::Error) -> Self {
        Self::new(error.into(), Option::<PathBuf>::None)
    }
}

#[derive(Debug)]
pub enum SnapshotErrorDetail {
    IoError { inner: io::Error },
    Lua { inner: rlua::Error },
    FileDidNotExist,
    FileNameBadUnicode,
    FileContentsBadUnicode { inner: std::str::Utf8Error },
    MalformedProject { inner: serde_json::Error },
}

impl From<io::Error> for SnapshotErrorDetail {
    fn from(inner: io::Error) -> Self {
        Self::IoError { inner }
    }
}

impl From<rlua::Error> for SnapshotErrorDetail {
    fn from(inner: rlua::Error) -> Self {
        Self::Lua { inner }
    }
}

impl SnapshotErrorDetail {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        use self::SnapshotErrorDetail::*;

        match self {
            IoError { inner } => Some(inner),
            Lua { inner } => Some(inner),
            FileContentsBadUnicode { inner } => Some(inner),
            MalformedProject { inner } => Some(inner),
            _ => None,
        }
    }
}

impl fmt::Display for SnapshotErrorDetail {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        use self::SnapshotErrorDetail::*;

        match self {
            IoError { inner } => write!(formatter, "I/O error: {}", inner),
            Lua { inner } => write!(formatter, "{}", inner),
            FileDidNotExist => write!(formatter, "file did not exist"),
            FileNameBadUnicode => write!(formatter, "file name had malformed Unicode"),
            FileContentsBadUnicode { inner } => {
                write!(formatter, "file had malformed unicode: {}", inner)
            }
            MalformedProject { inner } => write!(formatter, "{}", inner),
        }
    }
}
