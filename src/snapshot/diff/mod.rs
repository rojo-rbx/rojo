mod hash;
mod variant;

pub use hash::*;
pub use variant::*;

use rbx_dom_weak::{types::Ref, WeakDom};
use std::{collections::HashMap, hash::Hash};

pub fn diff_trees(dom_1: &WeakDom, dom_2: &WeakDom) -> Diff {
    let list_1 = hash_tree(dom_1);
    let mut list_2 = invert_map(hash_tree(dom_2));

    let mut removals = Vec::new();

    for (referent, hash) in list_1 {
        // If it's in both lists, we'll pull it out of list_2 so that
        // it doesn't get flagged as an addition.
        if list_2.contains_key(&hash) {
            list_2.remove(&hash);
        } else {
            removals.push(referent);
        }
    }

    Diff {
        removals,
        additions: list_2.into_iter().map(|(_, referent)| referent).collect(),
    }
}

fn invert_map<K: Hash + Eq, V: Hash + Eq>(map: HashMap<K, V>) -> HashMap<V, K> {
    map.into_iter().map(|(key, value)| (value, key)).collect()
}

pub struct Diff {
    /// Referents that were either removed or changed (in dom 1, not in dom 2)
    pub removals: Vec<Ref>,
    /// Referents that were added or changed (in dom 2, not in dom 1)
    pub additions: Vec<Ref>,
}

impl Diff {
    /// Returns the total number of diffs represented by this struct
    #[inline]
    pub fn total(&self) -> usize {
        self.removals.len() + self.additions.len()
    }
}
