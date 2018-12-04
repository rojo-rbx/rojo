use std::{
    path::{Path, PathBuf},
    collections::{HashMap, HashSet},
};

#[derive(Debug)]
struct PathMapNode<T> {
    value: T,
    children: HashSet<PathBuf>,
}

/// A map from paths to instance IDs, with a bit of additional data that enables
/// removing a path and all of its child paths from the tree in constant time.
#[derive(Debug)]
pub struct PathMap<T> {
    nodes: HashMap<PathBuf, PathMapNode<T>>,
}

impl<T> PathMap<T> {
    pub fn new() -> PathMap<T> {
        PathMap {
            nodes: HashMap::new(),
        }
    }

    pub fn get(&self, path: &Path) -> Option<&T> {
        self.nodes.get(path).map(|v| &v.value)
    }

    pub fn insert(&mut self, path: PathBuf, value: T) {
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

    pub fn remove(&mut self, root_path: &Path) -> Option<T> {
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
}