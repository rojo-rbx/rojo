use std::{
    fmt,
    error::Error,
    path::PathBuf,
};

use crate::{
    instance_snapshot::InstanceSnapshot,
};

pub type SnapshotResult<'a> = Result<Option<InstanceSnapshot<'a>>, SnapshotError>;

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

#[derive(Debug)]
pub enum SnapshotErrorDetail {
    FileDidNotExist,
    FileNameBadUnicode,
}

impl SnapshotErrorDetail {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

impl fmt::Display for SnapshotErrorDetail {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        use self::SnapshotErrorDetail::*;

        match self {
            FileDidNotExist => write!(formatter, "file did not exist"),
            FileNameBadUnicode => write!(formatter, "file name had malformed Unicode"),
        }
    }
}