use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{path_serializer, project::ProjectNode};

/// Rojo-specific metadata that can be associated with an instance or a snapshot
/// of an instance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstanceMetadata {
    /// Whether instances not present in the source should be ignored when
    /// live-syncing. This is useful when there are instances that Rojo does not
    /// manage.
    pub ignore_unknown_instances: bool,

    /// The paths that, when changed, could cause the function that generated
    /// this snapshot to generate a different snapshot. Paths should be included
    /// even if they don't exist, since the presence of a file can change the
    /// outcome of a snapshot function.
    ///
    /// The first path in this  list is considered the "instigating path", and
    /// will be the snapshot target if any of the contributing paths change.
    ///
    /// For example, a file named foo.lua might have these contributing paths:
    /// - foo.lua (instigating path)
    /// - foo.meta.json (even if this file doesn't exist!)
    ///
    /// A directory named bar/ included in the project file might have these:
    /// - bar/ (instigating path)
    /// - bar/init.meta.json
    /// - bar/init.lua
    /// - bar/init.server.lua
    /// - bar/init.client.lua
    /// - default.project.json
    ///
    /// This path is used to make sure that file changes update all instances
    /// that may need updates.
    // TODO: Change this to be a SmallVec for performance in common cases?
    #[serde(serialize_with = "path_serializer::serialize_vec_absolute")]
    pub contributing_paths: Vec<PathBuf>,

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
            contributing_paths: Vec::new(),
            project_node: None,
        }
    }
}
