use std::path::PathBuf;
use std::collections::HashMap;
use std::io::Read;
use std::fs::{self, File};

use id::{Id, get_id};
use file_route::FileRoute;
use partition::Partition;

/// Represents a file or directory that has been read from the filesystem.
#[derive(Debug, Clone)]
enum FileItem {
    File {
        contents: String,
        route: FileRoute,
    },
    Directory {
        children: HashMap<String, FileItem>,
        route: FileRoute,
    },
}

// TODO: Switch to enum to represent more value types
type RbxValue = String;

#[derive(Debug, Clone)]
struct RbxInstance {
    /// Maps to the `Name` property on Instance.
    pub name: String,

    /// Maps to the `ClassName` property on Instance.
    pub class_name: String,

    /// Maps to the `Parent` property on Instance.
    pub parent: Option<Id>,

    /// Contains all other properties of an Instance.
    pub properties: HashMap<String, RbxValue>,
}

struct RbxSession {
    pub partitions: HashMap<String, Partition>,

    /// The RbxInstance that represents each partition.
    pub partition_instances: HashMap<String, Id>,

    /// The in-memory files associated with each partition.
    pub partition_files: HashMap<String, FileItem>,

    /// The store of all instances in the session.
    pub instances: HashMap<Id, RbxInstance>,
    // pub instances_by_route: HashMap<FileRoute, Id>,
}

fn file_to_instance(file_item: &FileItem, partition: &Partition) -> (Id, HashMap<Id, RbxInstance>) {
    match file_item {
        &FileItem::File { ref contents, ref route } => {
            let mut properties = HashMap::new();
            properties.insert("Value".to_string(), contents.clone());

            let primary_id = get_id();
            let mut instances = HashMap::new();

            instances.insert(primary_id, RbxInstance {
                name: route.name(partition).to_string(),
                class_name: "StringValue".to_string(),
                parent: None,
                properties,
            });

            (primary_id, instances)
        },
        &FileItem::Directory { ref children, ref route } => {
            let primary_id = get_id();
            let mut instances = HashMap::new();

            instances.insert(primary_id, RbxInstance {
                name: route.name(partition).to_string(),
                class_name: "Folder".to_string(),
                parent: None,
                properties: HashMap::new(),
            });

            for child_file_item in children.values() {
                let (child_id, mut child_instances) = file_to_instance(child_file_item, partition);

                child_instances.get_mut(&child_id).unwrap().parent = Some(primary_id);

                for (instance_id, instance) in child_instances.drain() {
                    instances.insert(instance_id, instance);
                }
            }

            (primary_id, instances)
        }
    }
}

impl RbxSession {
    fn new() -> RbxSession {
        RbxSession {
            partitions: HashMap::new(),
            partition_instances: HashMap::new(),
            partition_files: HashMap::new(),
            instances: HashMap::new(),
        }
    }

    fn load_files(&mut self) {
        for partition_name in self.partitions.keys() {
            let route = FileRoute {
                partition: partition_name.clone(),
                route: vec![],
            };

            let file_item = self.read(&route).expect("Couldn't load partitions");

            self.partition_files.insert(partition_name.clone(), file_item);
        }
    }

    fn load_instances(&mut self) {
        for (partition_name, file_item) in &self.partition_files {
            let partition = self.partitions.get(partition_name).unwrap();
            let (root_id, mut instances) = file_to_instance(&file_item, partition);

            // there has to be an std method for this
            // oh well
            for (instance_id, instance) in instances.drain() {
                self.instances.insert(instance_id, instance);
            }

            self.partition_instances.insert(partition_name.clone(), root_id);
        }
    }

    fn read(&self, route: &FileRoute) -> Result<FileItem, ()> {
        let partition_path = &self.partitions.get(&route.partition)
            .ok_or(())?.path;
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
        let partition_path = &self.partitions.get(&route.partition)
            .ok_or(())?.path;
        let path = route.to_path_buf(partition_path);

        let mut file = File::open(path)
            .map_err(|_| ())?;

        let mut contents = String::new();

        file.read_to_string(&mut contents)
            .map_err(|_| ())?;

        Ok(FileItem::File {
            contents,
            route: route.clone(),
        })
    }

    fn read_directory(&self, route: &FileRoute) -> Result<FileItem, ()> {
        let partition_path = &self.partitions.get(&route.partition)
            .ok_or(())?.path;
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
            route: route.clone(),
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

        let partition = Partition {
            path: root_dir.path().to_path_buf(),
            target: vec!["ReplicatedStorage".to_string()],
        };

        session.partitions.insert("agh".to_string(), partition);

        session.load_files();

        assert_eq!(session.partition_files.len(), 1);

        {
            let folder = session.partition_files.values().nth(0).unwrap();

            let children = match folder {
                &FileItem::Directory { ref children, .. } => children,
                _ => panic!("Not a directory!"),
            };

            assert_eq!(children.len(), 2);
            assert!(children.get("foo.txt").is_some());
            assert!(children.get("bar.tsv").is_some());
        }

        session.load_instances();

        assert_eq!(session.instances.len(), 3);
        assert_eq!(session.partition_instances.len(), 1);

        {
            let folder_id = session.partition_instances.values().nth(0).unwrap();

            let folder = session.instances.get(folder_id).unwrap();
            assert_eq!(folder.name, "ReplicatedStorage");
        }
   }
}
