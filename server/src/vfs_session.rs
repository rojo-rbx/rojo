use std::collections::HashMap;
use std::io::Read;
use std::fs::{self, File};
use std::mem;

use file_route::FileRoute;
use session::SessionConfig;

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

impl FileItem {
    pub fn get_route(&self) -> &FileRoute {
        match self {
            FileItem::File { route, .. } => route,
            FileItem::Directory { route, .. } => route,
        }
    }
}

#[derive(Debug, Clone)]
pub enum FileChange {
    Created(FileRoute),
    Deleted(FileRoute),
    Updated(FileRoute),
    Moved(FileRoute, FileRoute),
}

pub struct VfsSession {
    pub config: SessionConfig,

    /// The in-memory files associated with each partition.
    pub partition_files: HashMap<String, FileItem>,
}

impl VfsSession {
    pub fn new(config: SessionConfig) -> VfsSession {
        VfsSession {
            config: config,
            partition_files: HashMap::new(),
        }
    }

    pub fn read_partitions(&mut self) {
        for partition_name in self.config.partitions.keys() {
            let route = FileRoute {
                partition: partition_name.clone(),
                route: Vec::new(),
            };

            let file_item = self.read(&route).expect("Couldn't load partitions");

            self.partition_files.insert(partition_name.clone(), file_item);
        }
    }

    pub fn handle_change(&mut self, change: &FileChange) -> Option<()> {
        println!("Got file change {:?}", change);

        match change {
            FileChange::Created(route) | FileChange::Updated(route) => {
                let new_item = self.read(&route).ok()?;
                self.set_file_item(new_item);
            },
            FileChange::Deleted(route) => {
                self.delete_route(&route);
            },
            FileChange::Moved(from_route, to_route) => {
                let new_item = self.read(&to_route).ok()?;
                self.delete_route(&from_route);
                self.set_file_item(new_item);
            },
        }

        None
    }

    pub fn get_by_route(&self, route: &FileRoute) -> Option<&FileItem> {
        let partition = self.partition_files.get(&route.partition)?;
        let mut current = partition;

        for piece in &route.route {
            match current {
                FileItem::File { .. } => return None,
                FileItem::Directory { children, .. } => {
                    current = children.get(piece)?;
                },
            }
        }

        Some(current)
    }

    pub fn get_by_route_mut(&mut self, route: &FileRoute) -> Option<&mut FileItem> {
        let mut current = self.partition_files.get_mut(&route.partition)?;

        for piece in &route.route {
            let mut next = match { current } {
                FileItem::File { .. } => return None,
                FileItem::Directory { children, .. } => {
                    children.get_mut(piece)?
                },
            };

            current = next;
        }

        Some(current)
    }

    pub fn set_file_item(&mut self, item: FileItem) {
        match self.get_by_route_mut(item.get_route()) {
            Some(existing) => {
                mem::replace(existing, item);
                return;
            },
            None => {},
        }

        if item.get_route().route.len() > 0 {
            let mut parent_route = item.get_route().clone();
            let child_name = parent_route.route.pop().unwrap();

            let mut parent_children = HashMap::new();
            parent_children.insert(child_name, item);

            let parent_item = FileItem::Directory {
                route: parent_route,
                children: parent_children,
            };

            self.set_file_item(parent_item);
        } else {
            self.partition_files.insert(item.get_route().partition.clone(), item);
        }
    }

    pub fn delete_route(&mut self, route: &FileRoute) -> Option<()> {
        if route.route.len() == 0 {
            self.partition_files.remove(&route.partition);
            return Some(());
        }

        let mut current = self.partition_files.get_mut(&route.partition)?;

        for i in 0..(route.route.len() - 1) {
            let piece = &route.route[i];

            let mut next = match { current } {
                FileItem::File { .. } => return None,
                FileItem::Directory { children, .. } => {
                    children.get_mut(piece)?
                },
            };

            current = next;
        }

        match current {
            FileItem::Directory { children, .. } => {
                children.remove(route.route.last().unwrap().as_str());
            },
            _ => {},
        }

        Some(())
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
