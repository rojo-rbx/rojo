use std::{io, path::PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SnapshotError {
    #[error("file name had malformed Unicode")]
    FileNameBadUnicode { path: PathBuf },

    #[error("file had malformed Unicode contents")]
    FileContentsBadUnicode {
        source: std::str::Utf8Error,
        path: PathBuf,
    },

    #[error("malformed project file")]
    MalformedProject {
        source: serde_json::Error,
        path: PathBuf,
    },

    #[error("malformed .model.json file")]
    MalformedModelJson {
        source: serde_json::Error,
        path: PathBuf,
    },

    #[error("malformed .meta.json file")]
    MalformedMetaJson {
        source: serde_json::Error,
        path: PathBuf,
    },

    #[error(transparent)]
    Io {
        #[from]
        source: io::Error,
    },
}

impl SnapshotError {
    pub(crate) fn file_name_bad_unicode(path: impl Into<PathBuf>) -> Self {
        Self::FileNameBadUnicode { path: path.into() }
    }

    pub(crate) fn file_contents_bad_unicode(
        source: std::str::Utf8Error,
        path: impl Into<PathBuf>,
    ) -> Self {
        Self::FileContentsBadUnicode {
            source,
            path: path.into(),
        }
    }

    pub(crate) fn malformed_project(source: serde_json::Error, path: impl Into<PathBuf>) -> Self {
        Self::MalformedProject {
            source,
            path: path.into(),
        }
    }

    pub(crate) fn malformed_model_json(
        source: serde_json::Error,
        path: impl Into<PathBuf>,
    ) -> Self {
        Self::MalformedModelJson {
            source,
            path: path.into(),
        }
    }

    pub(crate) fn malformed_meta_json(source: serde_json::Error, path: impl Into<PathBuf>) -> Self {
        Self::MalformedMetaJson {
            source,
            path: path.into(),
        }
    }
}
