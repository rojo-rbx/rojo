use std::path::{Path, PathBuf, Component};

use partition::Partition;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileRoute {
    pub partition: String,
    pub route: Vec<String>,
}

impl FileRoute {
    pub fn from_path(path: &Path, partition: &Partition) -> Option<FileRoute> {
        assert!(path.is_absolute());

        let relative_path = path.strip_prefix(&partition.path).ok()?;
        let mut route = Vec::new();

        for component in relative_path.components() {
            match component {
                Component::Normal(piece) => {
                    route.push(piece.to_string_lossy().into_owned());
                },
                _ => panic!("Unexpected path component: {:?}", component),
            }
        }

        Some(FileRoute {
            partition: partition.name.clone(),
            route,
        })
    }

    pub fn parent(&self) -> Option<FileRoute> {
        if self.route.len() == 0 {
            return None;
        }

        let mut new_route = self.route.clone();
        new_route.pop();

        Some(FileRoute {
            partition: self.partition.clone(),
            route: new_route,
        })
    }

    /// Creates a PathBuf out of the `FileRoute` based on the given partition
    /// `Path`.
    pub fn to_path_buf(&self, partition_path: &Path) -> PathBuf {
        let mut result = partition_path.to_path_buf();

        for route_piece in &self.route {
            result.push(route_piece);
        }

        result
    }

    /// Creates a version of the FileRoute with the given extra pieces appended
    /// to the end.
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
                // We're targeting the game!
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
