use std::{path::Path, str};

use maplit::hashmap;
use rbx_dom_weak::{RbxId, RbxTree, RbxValue};
use vfs::{IoResultExt, Vfs};

use crate::snapshot::{InstanceContext, InstanceMetadata, InstanceSnapshot};

use super::{
    error::SnapshotError,
    meta_file::AdjacentMetadata,
    middleware::{SnapshotInstanceResult, SnapshotMiddleware},
    util::match_file_name,
};

pub struct SnapshotTxt;

impl SnapshotMiddleware for SnapshotTxt {
    fn from_vfs(context: &InstanceContext, vfs: &Vfs, path: &Path) -> SnapshotInstanceResult {
        let meta = vfs.metadata(path)?;

        if meta.is_dir() {
            return Ok(None);
        }

        let instance_name = match match_file_name(path, ".txt") {
            Some(name) => name,
            None => return Ok(None),
        };

        let contents = vfs.read(path)?;
        let contents_str = str::from_utf8(&contents)
            .map_err(|err| SnapshotError::file_contents_bad_unicode(err, path))?
            .to_string();

        let properties = hashmap! {
            "Value".to_owned() => RbxValue::String {
                value: contents_str,
            },
        };

        let meta_path = path.with_file_name(format!("{}.meta.json", instance_name));

        let mut snapshot = InstanceSnapshot::new()
            .name(instance_name)
            .class_name("StringValue")
            .properties(properties)
            .metadata(
                InstanceMetadata::new()
                    .instigating_source(path)
                    .relevant_paths(vec![path.to_path_buf(), meta_path.clone()])
                    .context(context),
            );

        if let Some(meta_contents) = vfs.read(meta_path).with_not_found()? {
            let mut metadata = AdjacentMetadata::from_slice(&meta_contents);
            metadata.apply_all(&mut snapshot);
        }

        Ok(Some(snapshot))
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
            SnapshotTxt::from_vfs(&InstanceContext::default(), &mut vfs, &entry)
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
