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

    // TODO: Make source_path a SmallVec<PathBuf> in order to support meta
    // files? Maybe we should use another member of the snapshot middleware to
    // support damage-painting
    /// The path that this file came from, if it's the top-level instance from a
    /// model that came directly from disk.
    ///
    /// This path is used to make sure that file changes update all instances
    /// that may need updates..
    pub source_path: Option<PathBuf>,

    /// If this instance was defined in a project file, this is the name from
    /// the project file and the node under it.
    ///
    /// This information is used to make sure the instance has the correct name,
    /// project-added children, and metadata when it's updated in response to a
    /// file change.
    pub project_node: Option<(String, ProjectNode)>,
}

impl Default for InstanceMetadata {
    fn default() -> Self {
        InstanceMetadata {
            ignore_unknown_instances: false,
            source_path: None,
            project_node: None,
        }
    }
}
