use crate::snapshot::InstanceSnapshot;

use super::error::SnapshotError;

pub type SnapshotInstanceResult = Result<Option<InstanceSnapshot>, SnapshotError>;
