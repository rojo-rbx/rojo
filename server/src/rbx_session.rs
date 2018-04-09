use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::io::Read;
use std::fs::{self, File};

use id::{Id};

// TODO: Add lifetime, switch to using Cow<'a, str> instead of String? It's
// possible that it would be too cumbersome!
#[derive(Debug, Clone, PartialEq, Hash)]
struct FileRoute {
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
}

/// Represents a file or directory that has been read from the filesystem.
// TODO: Keep track of file Path or FileRoute?
#[derive(Debug, Clone)]
enum FileItem {
    File {
        contents: String,
    },
    Directory {
        children: HashMap<String, FileItem>,
    },
}

struct RbxInstance {
    pub class_name: String,
    pub parent: Option<Id>,
    pub properties: HashMap<String, String>,
}

struct RbxSession {
    pub partition_paths: HashMap<String, PathBuf>,
    pub partition_instances: HashMap<String, Id>,
    pub partition_files: HashMap<String, FileItem>,
    pub instances: HashMap<Id, RbxInstance>,
}

fn file_to_instance(file_item: &FileItem) -> RbxInstance {
    match file_item {
        &FileItem::File { ref contents } => {
            let mut properties = HashMap::new();
            properties.insert("Value".to_string(), contents.clone());

            RbxInstance {
                class_name: "StringValue".to_string(),
                parent: None,
                properties,
            }
        },
        &FileItem::Directory { ref children } => {
            RbxInstance {
                class_name: "Folder".to_string(),
                parent: None,
                properties: HashMap::new(),
            }
        }
    }
}

impl RbxSession {
    fn new() -> RbxSession {
        RbxSession {
            partition_paths: HashMap::new(),
            partition_instances: HashMap::new(),
            partition_files: HashMap::new(),
            instances: HashMap::new(),
        }
    }

    fn load_files(&mut self) {
        for partition_name in self.partition_paths.keys() {
            let route = FileRoute {
                partition: partition_name.clone(),
                route: vec![],
            };

            let file_item = self.read(&route).expect("Couldn't load partitions");

            self.partition_files.insert(partition_name.clone(), file_item);
        }
    }

    fn read(&self, route: &FileRoute) -> Result<FileItem, ()> {
        let partition_path = self.partition_paths.get(&route.partition)
            .ok_or(())?;
        let path = route.to_path_buf(partition_path);

        println!("Read {:?}, path {}", route, path.display());

        let metadata = fs::metadata(path)
            .map_err(|_| ())?;

        if metadata.is_dir() {
            self.read_directory(route)
        } else if metadata.is_file() {
            self.read_file(route)
        } else {
            Err(())
        }
    }

    fn read_file(&self, route: &FileRoute) -> Result<FileItem, ()> {
        let partition_path = self.partition_paths.get(&route.partition)
            .ok_or(())?;
        let path = route.to_path_buf(partition_path);

        let mut file = File::open(path)
            .map_err(|_| ())?;

        let mut contents = String::new();

        file.read_to_string(&mut contents)
            .map_err(|_| ())?;

        Ok(FileItem::File {
            contents,
        })
    }

    fn read_directory(&self, route: &FileRoute) -> Result<FileItem, ()> {
        let partition_path = self.partition_paths.get(&route.partition)
            .ok_or(())?;
        let path = route.to_path_buf(partition_path);

        let reader = fs::read_dir(path)
            .map_err(|_| ())?;

        let mut children = HashMap::new();

        for entry in reader {
            let entry = entry
                .map_err(|_| ())?;

            let path = entry.path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();

            let child_route = route.extended_with(&[&name]);

            let child_item = self.read(&child_route)?;

            children.insert(name, child_item);
        }

        Ok(FileItem::Directory {
            children,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile;

    // I'm not exactly sure how I want to structure these tests
    // Essentially, I need a bunch of random files to load, and to measure:
    // * What FileItems were loaded?
    // * Are changes logged to those FileItems correctly?
    // * What RbxInstance objects are generated from them?
    // * Are changes propagated from FileItem through to those RbxInstances?

    #[test]
    fn file_items_correct() {
        use std::io::Write;

        let root_dir = tempfile::tempdir().unwrap();

        let foo_path = root_dir.path().join("foo.txt");
        let bar_path = root_dir.path().join("bar.tsv");

        {
            let mut foo = File::create(foo_path).unwrap();
            writeln!(foo, "Hello, foo!").unwrap();

            let mut bar = File::create(bar_path).unwrap();
            writeln!(bar, "Hello, bar!").unwrap();
        }

        let mut session = RbxSession::new();

        session.partition_paths.insert("agh".to_string(), root_dir.path().to_path_buf());

        session.load_files();

        assert_eq!(session.partition_files.len(), 1);

        let folder = session.partition_files.values().nth(0).unwrap();

        let children = match folder {
            &FileItem::Directory { ref children } => children,
            _ => panic!("Not a directory!"),
        };

        assert_eq!(children.len(), 2);
        assert!(children.get("foo.txt").is_some());
        assert!(children.get("bar.tsv").is_some());
   }
}
