use std::{
    borrow::Borrow,
    collections::HashMap,
    fmt::{self, Debug},
    hash::Hash,
};

/// A map whose value contains a set of multiple values.
#[derive(Clone)]
pub struct MultiMap<K, V> {
    inner: HashMap<K, Vec<V>>,
}

#[allow(dead_code)] // This is a core library-ish struct, unused stuff is ok
impl<K: Hash + Eq, V: Eq> MultiMap<K, V> {
    pub fn new() -> Self {
        MultiMap {
            inner: HashMap::new(),
        }
    }

    pub fn get<Q: ?Sized>(&self, k: &Q) -> &[V]
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.inner.get(k).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn insert(&mut self, k: K, v: V) {
        let bucket = self.inner.entry(k).or_default();

        for value in &*bucket {
            if *value == v {
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

            if bucket.is_empty() {
                self.inner.remove(k.borrow());
            }

            removed_value
        } else {
            None
        }
    }
}

impl<K: Debug + Hash + Eq, V: Debug + Eq> Debug for MultiMap<K, V> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        self.inner.fmt(formatter)
    }
}

impl<K: Hash + Eq, V: Eq> PartialEq for MultiMap<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}
