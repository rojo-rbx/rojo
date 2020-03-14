use std::{error::Error, fmt, io, path::PathBuf};

use snafu::Snafu;

#[derive(Debug)]
pub struct SnapshotError {
    detail: SnapshotErrorDetail,
    path: Option<PathBuf>,
}

impl SnapshotError {
    pub fn new(detail: SnapshotErrorDetail, path: Option<impl Into<PathBuf>>) -> Self {
        Self {
            detail,
            path: path.map(Into::into),
        }
    }

    pub(crate) fn wrap(source: impl Into<SnapshotErrorDetail>, path: impl Into<PathBuf>) -> Self {
        Self {
            detail: source.into(),
            path: Some(path.into()),
        }
    }

    pub(crate) fn file_did_not_exist(path: impl Into<PathBuf>) -> Self {
        Self {
            detail: SnapshotErrorDetail::FileDidNotExist,
            path: Some(path.into()),
        }
    }

    pub(crate) fn file_name_bad_unicode(path: impl Into<PathBuf>) -> Self {
        Self {
            detail: SnapshotErrorDetail::FileNameBadUnicode,
            path: Some(path.into()),
        }
    }

    pub(crate) fn file_contents_bad_unicode(
        source: std::str::Utf8Error,
        path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            detail: SnapshotErrorDetail::FileContentsBadUnicode { source },
            path: Some(path.into()),
        }
    }

    pub(crate) fn malformed_project(source: serde_json::Error, path: impl Into<PathBuf>) -> Self {
        Self {
            detail: SnapshotErrorDetail::MalformedProject { source },
            path: Some(path.into()),
        }
    }

    pub(crate) fn malformed_model_json(
        source: serde_json::Error,
        path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            detail: SnapshotErrorDetail::MalformedModelJson { source },
            path: Some(path.into()),
        }
    }

    pub(crate) fn malformed_meta_json(source: serde_json::Error, path: impl Into<PathBuf>) -> Self {
        Self {
            detail: SnapshotErrorDetail::MalformedMetaJson { source },
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

impl From<io::Error> for SnapshotError {
    fn from(inner: io::Error) -> Self {
        Self::new(inner.into(), Option::<PathBuf>::None)
    }
}

impl From<rlua::Error> for SnapshotError {
    fn from(error: rlua::Error) -> Self {
        Self::new(error.into(), Option::<PathBuf>::None)
    }
}

#[derive(Debug, Snafu)]
pub enum SnapshotErrorDetail {
    #[snafu(display("I/O error"))]
    IoError { source: io::Error },

    #[snafu(display("Lua error"))]
    Lua { source: rlua::Error },

    #[snafu(display("file did not exist"))]
    FileDidNotExist,

    #[snafu(display("file name had malformed Unicode"))]
    FileNameBadUnicode,

    #[snafu(display("file had malformed Unicode contents"))]
    FileContentsBadUnicode { source: std::str::Utf8Error },

    #[snafu(display("malformed project file"))]
    MalformedProject { source: serde_json::Error },

    #[snafu(display("malformed .model.json file"))]
    MalformedModelJson { source: serde_json::Error },

    #[snafu(display("malformed .meta.json file"))]
    MalformedMetaJson { source: serde_json::Error },
}

impl From<io::Error> for SnapshotErrorDetail {
    fn from(source: io::Error) -> Self {
        SnapshotErrorDetail::IoError { source }
    }
}

impl From<rlua::Error> for SnapshotErrorDetail {
    fn from(source: rlua::Error) -> Self {
        SnapshotErrorDetail::Lua { source }
    }
}
