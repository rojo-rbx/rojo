use std::collections::BTreeMap;

#[non_exhaustive]
pub enum VfsSnapshot {
    File {
        contents: Vec<u8>,
    },

    Dir {
        children: BTreeMap<String, VfsSnapshot>,
    },
}

impl VfsSnapshot {
    pub fn file<C: Into<Vec<u8>>>(contents: C) -> Self {
        Self::File {
            contents: contents.into(),
        }
    }

    pub fn dir<K: Into<String>, I: IntoIterator<Item = (K, VfsSnapshot)>>(children: I) -> Self {
        Self::Dir {
            children: children
                .into_iter()
                .map(|(key, value)| (key.into(), value))
                .collect(),
        }
    }
}
