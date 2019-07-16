use std::path::Path;

use super::legacy::ImfsItem;

pub trait ImfsTrait {
    fn get<P: AsRef<Path>>(&self, path: P) -> Option<ImfsItem>;
    fn insert<P: AsRef<Path>>(&self, path: P, item: ImfsItem) -> Option<ImfsItem>;
    fn remove<P: AsRef<Path>>(&self, path: P) -> Option<ImfsItem>;
}