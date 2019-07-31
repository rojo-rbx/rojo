use std::{
    borrow::Cow,
    str,
};

use maplit::hashmap;
use rbx_dom_weak::{RbxTree, RbxValue, RbxId};

use crate::{
    imfs::new::{Imfs, ImfsSnapshot, FileSnapshot, ImfsFetcher, ImfsEntry},
    snapshot::InstanceSnapshot,
};

use super::{
    middleware::{SnapshotMiddleware, SnapshotInstanceResult, SnapshotFileResult},
};

pub struct SnapshotTxt;

impl SnapshotMiddleware for SnapshotTxt {
    fn from_imfs<F: ImfsFetcher>(
        imfs: &mut Imfs<F>,
        entry: ImfsEntry,
    ) -> SnapshotInstanceResult {
        if entry.is_directory() {
            return Ok(None);
        }

        let extension = match entry.path().extension() {
            Some(x) => x.to_str().unwrap(),
            None => return Ok(None),
        };

        if extension != "txt" {
            return Ok(None);
        }

        let instance_name = entry.path()
            .file_stem().expect("Could not extract file stem")
            .to_str().unwrap().to_string();

        let contents = entry.contents(imfs)?;
        let contents_str = str::from_utf8(contents)
            .expect("File content was not valid UTF-8").to_string();

        let properties = hashmap! {
            "Value".to_owned() => RbxValue::String {
                value: contents_str,
            },
        };

        Ok(Some(InstanceSnapshot {
            snapshot_id: None,
            name: Cow::Owned(instance_name),
            class_name: Cow::Borrowed("StringValue"),
            properties,
            children: Vec::new(),
        }))
    }

    fn from_instance(
        tree: &RbxTree,
        id: RbxId,
    ) -> SnapshotFileResult {
        let instance = tree.get_instance(id).unwrap();

        if instance.class_name != "StringValue" {
            return None;
        }

        if !instance.get_children_ids().is_empty() {
            return None;
        }

        let value = match instance.properties.get("Value") {
            Some(RbxValue::String { value }) => value.clone(),
            Some(_) => panic!("wrong type ahh"),
            None => String::new(),
        };

        let snapshot = ImfsSnapshot::File(FileSnapshot {
            contents: value.into_bytes(),
        });

        Some((instance.name.clone(), snapshot))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn smoke_test() {
        // TODO
    }
}