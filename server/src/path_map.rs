use std::{
    path::{self, Path, PathBuf},
    collections::{HashMap, HashSet},
};

use serde::Serialize;
use log::warn;

#[derive(Debug, Serialize)]
struct PathMapNode<T> {
    value: T,
    children: HashSet<PathBuf>,
}

/// A map from paths to another type, like instance IDs, with a bit of
/// additional data that enables removing a path and all of its child paths from
/// the tree more quickly.
#[derive(Debug, Serialize)]
pub struct PathMap<T> {
    nodes: HashMap<PathBuf, PathMapNode<T>>,
}

impl<T> Default for PathMap<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> PathMap<T> {
    pub fn new() -> PathMap<T> {
        PathMap {
            nodes: HashMap::new(),
        }
    }

    pub fn get(&self, path: impl AsRef<Path>) -> Option<&T> {
        self.nodes.get(path.as_ref()).map(|v| &v.value)
    }

    pub fn get_mut(&mut self, path: impl AsRef<Path>) -> Option<&mut T> {
        self.nodes.get_mut(path.as_ref()).map(|v| &mut v.value)
    }

    pub fn children(&self, path: impl AsRef<Path>) -> Option<Vec<&Path>> {
        self.nodes.get(path.as_ref()).map(|v| v.children.iter().map(AsRef::as_ref).collect())
    }

    pub fn insert(&mut self, path: impl Into<PathBuf>, value: T) {
        let path = path.into();

        if let Some(parent_path) = path.parent() {
            if let Some(parent) = self.nodes.get_mut(parent_path) {
                parent.children.insert(path.to_path_buf());
            }
        }

        self.nodes.insert(path, PathMapNode {
            value,
            children: HashSet::new(),
        });
    }

    pub fn remove(&mut self, root_path: impl AsRef<Path>) -> Option<T> {
        let root_path = root_path.as_ref();

        if let Some(parent_path) = root_path.parent() {
            if let Some(parent) = self.nodes.get_mut(parent_path) {
                parent.children.remove(root_path);
            }
        }

        let mut root_node = match self.nodes.remove(root_path) {
            Some(node) => node,
            None => return None,
        };

        let root_value = root_node.value;
        let mut to_visit: Vec<PathBuf> = root_node.children.drain().collect();

        while let Some(path) = to_visit.pop() {
            match self.nodes.remove(&path) {
                Some(mut node) => {
                    for child in node.children.drain() {
                        to_visit.push(child);
                    }
                },
                None => {
                    warn!("Consistency issue; tried to remove {} but it was already removed", path.display());
                },
            }
        }

        Some(root_value)
    }

    /// Traverses the route between `start_path` and `target_path` and returns
    /// the path closest to `target_path` in the tree.
    ///
    /// This is useful when trying to determine what paths need to be marked as
    /// altered when a change to a path is registered. Depending on the order of
    /// FS events, a file remove event could be followed by that file's
    /// directory being removed, in which case we should process that
    /// directory's parent.
    pub fn descend(&self, start_path: impl Into<PathBuf>, target_path: impl AsRef<Path>) -> PathBuf {
        let start_path = start_path.into();
        let target_path = target_path.as_ref();

        let relative_path = target_path.strip_prefix(&start_path)
            .expect("target_path did not begin with start_path");
        let mut current_path = start_path;

        for component in relative_path.components() {
            match component {
                path::Component::Normal(name) => {
                    let next_path = current_path.join(name);

                    if self.nodes.contains_key(&next_path) {
                        current_path = next_path;
                    } else {
                        return current_path;
                    }
                },
                _ => unreachable!(),
            }
        }

        current_path
    }
}