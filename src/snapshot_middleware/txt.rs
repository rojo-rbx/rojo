use std::{borrow::Cow, str};

use maplit::hashmap;
use rbx_dom_weak::{RbxId, RbxTree, RbxValue};

use crate::{
    imfs::{FileSnapshot, Imfs, ImfsEntry, ImfsFetcher, ImfsSnapshot},
    snapshot::{InstanceMetadata, InstanceSnapshot},
};

use super::middleware::{SnapshotFileResult, SnapshotInstanceResult, SnapshotMiddleware};

pub struct SnapshotTxt;

impl SnapshotMiddleware for SnapshotTxt {
    fn from_imfs<F: ImfsFetcher>(
        imfs: &mut Imfs<F>,
        entry: &ImfsEntry,
    ) -> SnapshotInstanceResult<'static> {
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

        let instance_name = entry
            .path()
            .file_stem()
            .expect("Could not extract file stem")
            .to_str()
            .unwrap()
            .to_string();

        let contents = entry.contents(imfs)?;
        let contents_str = str::from_utf8(contents)
            .expect("File content was not valid UTF-8")
            .to_string();

        let properties = hashmap! {
            "Value".to_owned() => RbxValue::String {
                value: contents_str,
            },
        };

        Ok(Some(InstanceSnapshot {
            snapshot_id: None,
            metadata: InstanceMetadata {
                relevant_paths: vec![entry.path().to_path_buf()],
                ..Default::default()
            },
            name: Cow::Owned(instance_name),
            class_name: Cow::Borrowed("StringValue"),
            properties,
            children: Vec::new(),
        }))
    }

    fn from_instance(tree: &RbxTree, id: RbxId) -> SnapshotFileResult {
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

        let mut file_name = instance.name.clone();
        file_name.push_str(".txt");

        Some((file_name, snapshot))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use insta::assert_yaml_snapshot;
    use maplit::hashmap;
    use rbx_dom_weak::RbxInstanceProperties;

    use crate::imfs::{ImfsDebug, NoopFetcher};

    #[test]
    fn instance_from_imfs() {
        let mut imfs = Imfs::new(NoopFetcher);
        let file = ImfsSnapshot::file("Hello there!");

        imfs.debug_load_snapshot("/foo.txt", file);

        let entry = imfs.get("/foo.txt").unwrap();
        let instance_snapshot = SnapshotTxt::from_imfs(&mut imfs, &entry).unwrap().unwrap();

        assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn imfs_from_instance() {
        let tree = RbxTree::new(string_value("Root", "Hello, world!"));
        let root_id = tree.get_root_id();

        let (_file_name, _file) = SnapshotTxt::from_instance(&tree, root_id).unwrap();
    }

    fn folder(name: impl Into<String>) -> RbxInstanceProperties {
        RbxInstanceProperties {
            name: name.into(),
            class_name: "Folder".to_owned(),
            properties: Default::default(),
        }
    }

    fn string_value(name: impl Into<String>, value: impl Into<String>) -> RbxInstanceProperties {
        RbxInstanceProperties {
            name: name.into(),
            class_name: "StringValue".to_owned(),
            properties: hashmap! {
                "Value".to_owned() => RbxValue::String {
                    value: value.into(),
                },
            },
        }
    }
}
