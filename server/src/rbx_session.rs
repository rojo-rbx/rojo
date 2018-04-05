use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::io::Read;
use std::fs::{self, File};

use id::{Id};

#[derive(Debug, Clone)]
struct FileRoute {
    partition: String,
    route: Vec<String>,
}

impl FileRoute {
    pub fn to_path_buf(&self, partition_path: &Path) -> PathBuf {
        let mut result = partition_path.to_path_buf();

        for route_piece in &self.route {
            result.push(route_piece);
        }

        result
    }

    pub fn extended_with(&self, pieces: &[&str]) -> FileRoute {
        let mut result = self.clone();

        for piece in pieces {
            result.route.push(piece.to_string());
        }

        result
    }
}

enum FileItem {
    File {
        contents: String,
    },
    Directory {
        children: HashMap<String, FileItem>,
    },
}

struct RbxInstance {
    parent: Option<Id>,
}

struct RbxSession {
    partitions_path: HashMap<String, PathBuf>,
    partition_instances: HashMap<String, Id>,
}

impl RbxSession {
    fn load(&self) {
        for (partition_name, partition_path) in &self.partitions_path {
        }
    }

    fn read(&self, route: &FileRoute) -> Result<FileItem, ()> {
        let partition_path = self.partitions_path.get(&route.partition)
            .ok_or(())?;
        let path = route.to_path_buf(partition_path);

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
        let partition_path = self.partitions_path.get(&route.partition)
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
        let partition_path = self.partitions_path.get(&route.partition)
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

            match self.read(&child_route) {
                Ok(child_item) => {
                    children.insert(name, child_item);
                },
                Err(_) => {},
            }
        }

        Ok(FileItem::Directory {
            children,
        })
    }
}
