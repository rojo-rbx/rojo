use std::path::{Path, PathBuf, Component};

use partition::Partition;

// TODO: Change backing data structure to use a single allocation with slices
// taken out of it for each portion
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileRoute {
    pub partition: String,
    pub route: Vec<String>,
}

impl FileRoute {
    pub fn from_path(path: &Path, partition: &Partition) -> Option<FileRoute> {
        assert!(path.is_absolute());
        assert!(path.starts_with(&partition.path));

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

    pub fn file_name(&self, partition: &Partition) -> String {
        if self.route.len() == 0 {
            partition.path.file_name().unwrap().to_str().unwrap().to_string()
        } else {
            self.route.last().unwrap().clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    const ROOT_PATH: &'static str = "C:\\";

    #[cfg(not(windows))]
    const ROOT_PATH: &'static str = "/";

    #[test]
    fn from_path_empty() {
        let path = Path::new(ROOT_PATH).join("a/b/c");

        let partition = Partition {
            name: "foo".to_string(),
            path: path.clone(),
            target: vec![],
        };

        let route = FileRoute::from_path(&path, &partition);

        assert_eq!(route, Some(FileRoute {
            partition: "foo".to_string(),
            route: vec![],
        }));
    }

    #[test]
    fn from_path_non_empty() {
        let partition_path = Path::new(ROOT_PATH).join("a/b/c");

        let inside_path = partition_path.join("d");

        let partition = Partition {
            name: "bar".to_string(),
            path: partition_path,
            target: vec![],
        };

        let route = FileRoute::from_path(&inside_path, &partition);

        assert_eq!(route, Some(FileRoute {
            partition: "bar".to_string(),
            route: vec!["d".to_string()],
        }));
    }

    #[test]
    fn file_name_empty_route() {
        let partition_path = Path::new(ROOT_PATH).join("a/b/c");

        let partition = Partition {
            name: "bar".to_string(),
            path: partition_path,
            target: vec![],
        };

        let route = FileRoute {
            partition: "bar".to_string(),
            route: vec![],
        };

        assert_eq!(route.file_name(&partition), "c");
    }

    #[test]
    fn file_name_non_empty_route() {
        let partition_path = Path::new(ROOT_PATH).join("a/b/c");

        let partition = Partition {
            name: "bar".to_string(),
            path: partition_path,
            target: vec![],
        };

        let route = FileRoute {
            partition: "bar".to_string(),
            route: vec!["foo".to_string(), "hello.lua".to_string()],
        };

        assert_eq!(route.file_name(&partition), "hello.lua");
    }
}