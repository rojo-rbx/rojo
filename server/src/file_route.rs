use std::path::{Path, PathBuf};

use partition::Partition;

// TODO: Add lifetime, switch to using Cow<'a, str> instead of String? It's
// possible that it would be too cumbersome!
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct FileRoute {
    pub partition: String,
    pub route: Vec<String>,
}

impl FileRoute {
    /// Creates a PathBuf out of the `FileRoute` based on the given partition
    /// `Path`.
    // TODO: Tests
    pub fn to_path_buf(&self, partition_path: &Path) -> PathBuf {
        let mut result = partition_path.to_path_buf();

        for route_piece in &self.route {
            result.push(route_piece);
        }

        result
    }

    /// Creates a version of the FileRoute with the given extra pieces appended
    /// to the end.
    // TODO: Test
    pub fn extended_with(&self, pieces: &[&str]) -> FileRoute {
        let mut result = self.clone();

        for piece in pieces {
            result.route.push(piece.to_string());
        }

        result
    }

    /// This function is totally wrong and should be handled by middleware, heh.
    pub fn name(&self, partition: &Partition) -> String { // I guess??
        if self.route.len() == 0 {
            // This FileRoute refers to the partition itself

            if partition.target.len() == 0 {
                // We're targeting the game itself!
                "game".to_string()
            } else {
                partition.target.last().unwrap().clone()
            }
        } else {
            // This FileRoute refers to an item in a partition
            self.route.last().unwrap().clone()
        }
    }
}
