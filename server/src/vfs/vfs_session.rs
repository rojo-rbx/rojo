use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Instant;

use middleware::MiddlewareChain;
use vfs::VfsItem;

/// Represents a virtual layer over multiple parts of the filesystem.
///
/// Paths in this system are represented as slices of strings, and are always
/// relative to a partition, which is an absolute path into the real filesystem.
pub struct VfsSession {
    /// Contains all of the partitions mounted by the Vfs.
    ///
    /// These must be absolute paths!
    partitions: HashMap<String, PathBuf>,

    /// A chronologically-sorted list of routes that changed since the Vfs was
    /// created, along with a timestamp denoting when.
    change_history: Vec<VfsChange>,

    /// When the Vfs was initialized; used for change tracking.
    start_time: Instant,

    middleware_chain: &'static MiddlewareChain,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VfsChange {
    timestamp: f64,
    route: Vec<String>,
}

impl VfsSession {
    pub fn new(middleware_chain: &'static MiddlewareChain) -> VfsSession {
        VfsSession {
            partitions: HashMap::new(),
            start_time: Instant::now(),
            change_history: Vec::new(),
            middleware_chain,
        }
    }

    pub fn get_partitions(&self) -> &HashMap<String, PathBuf> {
        &self.partitions
    }

    pub fn insert_partition<P: Into<PathBuf>>(&mut self, name: &str, path: P) {
        let path = path.into();

        assert!(path.is_absolute());

        self.partitions.insert(name.to_string(), path.into());
    }

    fn route_to_path(&self, route: &[String]) -> Option<PathBuf> {
        let (partition_name, rest) = match route.split_first() {
            Some((first, rest)) => (first, rest),
            None => return None,
        };

        let partition = match self.partitions.get(partition_name) {
            Some(v) => v,
            None => return None,
        };

        // It's possible that the partition points to a file if `rest` is empty.
        // Joining "" onto a path will put a trailing slash on, which causes
        // file reads to fail.
        let full_path = if rest.is_empty() {
            partition.clone()
        } else {
            let joined = rest.join("/");
            let relative = Path::new(&joined);

            partition.join(relative)
        };

        Some(full_path)
    }

    fn read_dir<P: AsRef<Path>>(&self, route: &[String], path: P) -> Result<VfsItem, ()> {
        let path = path.as_ref();
        let reader = match fs::read_dir(path) {
            Ok(v) => v,
            Err(_) => return Err(()),
        };

        let mut children = HashMap::new();

        for entry in reader {
            let entry = match entry {
                Ok(v) => v,
                Err(_) => return Err(()),
            };

            let path = entry.path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();

            let mut child_route = route.iter().cloned().collect::<Vec<_>>();
            child_route.push(name.clone());

            match self.read_path(&child_route, &path) {
                Ok(child_item) => {
                    children.insert(name, child_item);
                },
                Err(_) => {},
            }
        }

        Ok(VfsItem::Dir {
            route: route.iter().cloned().collect::<Vec<_>>(),
            children,
        })
    }

    fn read_file<P: AsRef<Path>>(&self, route: &[String], path: P) -> Result<VfsItem, ()> {
        let path = path.as_ref();
        let mut file = match File::open(path) {
            Ok(v) => v,
            Err(_) => return Err(()),
        };

        let mut contents = String::new();

        match file.read_to_string(&mut contents) {
            Ok(_) => {},
            Err(_) => return Err(()),
        }

        Ok(VfsItem::File {
            route: route.iter().cloned().collect::<Vec<_>>(),
            contents,
        })
    }

    fn read_path<P: AsRef<Path>>(&self, route: &[String], path: P) -> Result<VfsItem, ()> {
        let path = path.as_ref();

        let metadata = match fs::metadata(path) {
            Ok(v) => v,
            Err(_) => return Err(()),
        };

        if metadata.is_dir() {
            self.read_dir(route, path)
        } else if metadata.is_file() {
            self.read_file(route, path)
        } else {
            Err(())
        }
    }

    /// Get the current time, used for logging timestamps for file changes.
    pub fn current_time(&self) -> f64 {
        let elapsed = self.start_time.elapsed();

        elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 * 1e-9
    }

    /// Register a new change to the filesystem at the given timestamp and VFS
    /// route.
    pub fn add_change(&mut self, timestamp: f64, route: Vec<String>) {
        match self.middleware_chain.handle_file_change(&route) {
            Some(routes) => {
                for route in routes {
                    self.change_history.push(VfsChange {
                        timestamp,
                        route,
                    });
                }
            },
            None => {}
        }
    }

    /// Collect a list of changes that occured since the given timestamp.
    pub fn changes_since(&self, timestamp: f64) -> &[VfsChange] {
        let mut marker: Option<usize> = None;

        for (index, value) in self.change_history.iter().enumerate().rev() {
            if value.timestamp >= timestamp {
                marker = Some(index);
            } else {
                break;
            }
        }

        if let Some(index) = marker {
            &self.change_history[index..]
        } else {
            &self.change_history[..0]
        }
    }

    /// Read an item from the filesystem using the given VFS route.
    pub fn read(&self, route: &[String]) -> Result<VfsItem, ()> {
        match self.route_to_path(route) {
            Some(path) => self.read_path(route, &path),
            None => Err(()),
        }
    }

    pub fn write(&self, _route: &[String], _item: VfsItem) -> Result<(), ()> {
        unimplemented!()
    }

    pub fn delete(&self, _route: &[String]) -> Result<(), ()> {
        unimplemented!()
    }
}
