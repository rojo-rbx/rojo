use std::{io, path::PathBuf};

use thiserror::Error;

use crate::project::ProjectError;

#[derive(Debug, Error)]
pub enum SnapshotError {
    #[error("file name had malformed Unicode")]
    FileNameBadUnicode { path: PathBuf },

    #[error("file had malformed Unicode contents at path {}", .path.display())]
    FileContentsBadUnicode {
        source: std::str::Utf8Error,
        path: PathBuf,
    },

    #[error("malformed project file at path {}", .path.display())]
    MalformedProject { source: ProjectError, path: PathBuf },

    #[error("malformed .model.json file at path {}", .path.display())]
    MalformedModelJson {
        source: serde_json::Error,
        path: PathBuf,
    },

    #[error("malformed .meta.json file at path {}", .path.display())]
    MalformedMetaJson {
        source: serde_json::Error,
        path: PathBuf,
    },

    #[error("malformed JSON at path {}", .path.display())]
    MalformedJson {
        source: serde_json::Error,
        path: PathBuf,
    },

    #[error("malformed CSV localization data at path {}", .path.display())]
    MalformedLocalizationCsv { source: csv::Error, path: PathBuf },

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

    pub(crate) fn malformed_project(source: ProjectError, path: impl Into<PathBuf>) -> Self {
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

    pub(crate) fn malformed_json(source: serde_json::Error, path: impl Into<PathBuf>) -> Self {
        Self::MalformedJson {
            source,
            path: path.into(),
        }
    }

    pub(crate) fn malformed_l10n_csv(source: csv::Error, path: impl Into<PathBuf>) -> Self {
        Self::MalformedLocalizationCsv {
            source,
            path: path.into(),
        }
    }
}
