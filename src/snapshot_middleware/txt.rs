use std::str;

use maplit::hashmap;
use rbx_dom_weak::{RbxId, RbxTree, RbxValue};

use crate::{
    snapshot::{InstanceContext, InstanceMetadata, InstanceSnapshot},
    vfs::{FileSnapshot, FsResultExt, Vfs, VfsEntry, VfsFetcher, VfsSnapshot},
};

use super::{
    error::SnapshotError,
    meta_file::AdjacentMetadata,
    middleware::{SnapshotFileResult, SnapshotInstanceResult, SnapshotMiddleware},
    util::match_file_name,
};

pub struct SnapshotTxt;

impl SnapshotMiddleware for SnapshotTxt {
    fn from_vfs<F: VfsFetcher>(
        _context: &mut InstanceContext,
        vfs: &Vfs<F>,
        entry: &VfsEntry,
    ) -> SnapshotInstanceResult {
        if entry.is_directory() {
            return Ok(None);
        }

        let instance_name = match match_file_name(entry.path(), ".txt") {
            Some(name) => name,
            None => return Ok(None),
        };

        let contents = entry.contents(vfs)?;
        let contents_str = str::from_utf8(&contents)
            .map_err(|err| SnapshotError::file_contents_bad_unicode(err, entry.path()))?
            .to_string();

        let properties = hashmap! {
            "Value".to_owned() => RbxValue::String {
                value: contents_str,
            },
        };

        let meta_path = entry
            .path()
            .with_file_name(format!("{}.meta.json", instance_name));

        let mut snapshot = InstanceSnapshot::new()
            .name(instance_name)
            .class_name("StringValue")
            .properties(properties)
            .metadata(
                InstanceMetadata::new()
                    .instigating_source(entry.path())
                    .relevant_paths(vec![entry.path().to_path_buf(), meta_path.clone()]),
            );

        if let Some(meta_entry) = vfs.get(meta_path).with_not_found()? {
            let meta_contents = meta_entry.contents(vfs)?;
            let mut metadata = AdjacentMetadata::from_slice(&meta_contents);
            metadata.apply_all(&mut snapshot);
        }

        Ok(Some(snapshot))
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

        let snapshot = VfsSnapshot::File(FileSnapshot {
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

    use crate::vfs::{NoopFetcher, VfsDebug};

    #[test]
    fn instance_from_vfs() {
        let mut vfs = Vfs::new(NoopFetcher);
        let file = VfsSnapshot::file("Hello there!");

        vfs.debug_load_snapshot("/foo.txt", file);

        let entry = vfs.get("/foo.txt").unwrap();
        let instance_snapshot =
            SnapshotTxt::from_vfs(&mut InstanceContext::default(), &mut vfs, &entry)
                .unwrap()
                .unwrap();

        assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn vfs_from_instance() {
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
