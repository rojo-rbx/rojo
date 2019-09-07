use std::{
    borrow::Borrow,
    collections::HashMap,
    fmt::{self, Debug},
    hash::Hash,
};

/// A map whose value contains a set of multiple values.
#[derive(Clone)]
pub struct MapSet<K, V> {
    inner: HashMap<K, Vec<V>>,
}

#[allow(dead_code)] // This is a core library-ish struct, unused stuff is ok
impl<K: Hash + Eq, V: Eq> MapSet<K, V> {
    pub fn new() -> Self {
        MapSet {
            inner: HashMap::new(),
        }
    }

    pub fn get<Q: Borrow<K>>(&mut self, k: Q) -> &[V] {
        self.inner.get(k.borrow()).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn insert(&mut self, k: K, v: V) {
        let bucket = self.inner.entry(k).or_default();

        for value in &*bucket {
            if &*value == &v {
                return;
            }
        }

        bucket.push(v);
    }

    pub fn remove<Q: Borrow<K>, U: Borrow<V>>(&mut self, k: Q, v: U) -> Option<V> {
        let b = v.borrow();

        if let Some(bucket) = self.inner.get_mut(k.borrow()) {
            let mut removed_value = None;

            if let Some(index) = bucket.iter().position(|value| value == b) {
                removed_value = Some(bucket.swap_remove(index));
            }

            if bucket.len() == 0 {
                self.inner.remove(k.borrow());
            }

            removed_value
        } else {
            None
        }
    }
}

impl<K: Debug + Hash + Eq, V: Debug + Eq> Debug for MapSet<K, V> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        self.inner.fmt(formatter)
    }
}

impl<K: Hash + Eq, V: Eq> PartialEq for MapSet<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}
