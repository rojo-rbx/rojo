use std::collections::HashMap;
use std::io::Read;
use std::fs::{self, File};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher, watcher};

use file_route::FileRoute;
use session_config::SessionConfig;

struct FileWatcher {
    watcher: RecommendedWatcher,
    rx: Receiver<DebouncedEvent>,
}

/// Represents a file or directory that has been read from the filesystem.
#[derive(Debug, Clone)]
pub enum FileItem {
    File {
        contents: String,
        route: FileRoute,
    },
    Directory {
        children: HashMap<String, FileItem>,
        route: FileRoute,
    },
}

pub struct FsSession {
    pub config: SessionConfig,

    /// The in-memory files associated with each partition.
    pub partition_files: HashMap<String, FileItem>,

    watchers: HashMap<String, FileWatcher>,
}

impl FsSession {
    pub fn new(config: SessionConfig) -> FsSession {
        FsSession {
            config: config,
            partition_files: HashMap::new(),
            watchers: HashMap::new(),
        }
    }

    pub fn init(&mut self) {
        self.load_partitions();
        self.watch_partitions();
    }

    pub fn step(&mut self) {
        for (partition_name, watcher) in self.watchers.iter() {
            let change_event = match watcher.rx.try_recv() {
                Ok(v) => v,
                Err(_) => continue,
            };

            println!("Change event on partition {}: {:?}", partition_name, change_event);
        }
    }

    fn watch_partitions(&mut self) {
        for (partition_name, partition) in self.config.partitions.iter() {
            let (tx, rx) = channel();

            let mut watcher = watcher(tx, Duration::from_millis(300)).unwrap();

            watcher.watch(&partition.path, RecursiveMode::Recursive).unwrap();

            self.watchers.insert(partition_name.clone(), FileWatcher {
                watcher,
                rx,
            });
        }
    }

    fn load_partitions(&mut self) {
        for partition_name in self.config.partitions.keys() {
            let route = FileRoute {
                partition: partition_name.clone(),
                route: vec![],
            };

            let file_item = self.read(&route).expect("Couldn't load partitions");

            self.partition_files.insert(partition_name.clone(), file_item);
        }
    }

    fn read(&self, route: &FileRoute) -> Result<FileItem, ()> {
        let partition_path = &self.config.partitions.get(&route.partition)
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
        let partition_path = &self.config.partitions.get(&route.partition)
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
        let partition_path = &self.config.partitions.get(&route.partition)
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
