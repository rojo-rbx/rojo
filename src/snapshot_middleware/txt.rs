use std::{path::Path, str};

use maplit::hashmap;
use rbx_dom_weak::RbxValue;
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

    use vfs::{InMemoryFs, VfsSnapshot};

    #[test]
    fn instance_from_vfs() {
        let mut imfs = InMemoryFs::new();
        let mut vfs = Vfs::new(imfs.clone());
        let file = VfsSnapshot::file("Hello there!");

        imfs.load_snapshot("/foo.txt", file).unwrap();

        let instance_snapshot =
            SnapshotTxt::from_vfs(&InstanceContext::default(), &mut vfs, Path::new("/foo.txt"))
                .unwrap()
                .unwrap();

        insta::assert_yaml_snapshot!(instance_snapshot);
    }
}
