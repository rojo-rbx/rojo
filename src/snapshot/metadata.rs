use std::path::PathBuf;

use crate::project::ProjectNode;

/// Rojo-specific metadata that can be associated with an instance or a snapshot
/// of an instance.
#[derive(Debug, Clone, PartialEq)]
pub struct InstanceMetadata {
    /// Whether instances not present in the source should be ignored when
    /// live-syncing. This is useful when there are instances that Rojo does not
    /// manage.
    pub ignore_unknown_instances: bool,

    /// A complete view of where this snapshot came from. It should contain
    /// enough information, if not None, to recreate this snapshot
    /// deterministically assuming the source has not changed state.
    pub source: Option<InstanceSource>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InstanceSource {
    File {
        path: PathBuf,
    },
    ProjectFile {
        path: PathBuf,
        name: String,
        node: ProjectNode,
    },
}

impl Default for InstanceMetadata {
    fn default() -> Self {
        InstanceMetadata {
            ignore_unknown_instances: false,
            source: None,
        }
    }
}
