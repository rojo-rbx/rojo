use std::path::Path;

use maplit::hashmap;
use memofs::{IoResultExt, Vfs};
use rbx_dom_weak::RbxValue;

use crate::{
    lua_ast::Statement,
    snapshot::{InstanceContext, InstanceMetadata, InstanceSnapshot},
};

use super::{
    error::SnapshotError,
    meta_file::AdjacentMetadata,
    middleware::{SnapshotInstanceResult, SnapshotMiddleware},
    util::match_file_name,
};

/// Catch-all middleware for snapshots on JSON files that aren't used for other
/// features, like Rojo projects, JSON models, or meta files.
pub struct SnapshotJson;

impl SnapshotMiddleware for SnapshotJson {
    fn from_vfs(context: &InstanceContext, vfs: &Vfs, path: &Path) -> SnapshotInstanceResult {
        let meta = vfs.metadata(path)?;

        if meta.is_dir() {
            return Ok(None);
        }

        let instance_name = match match_file_name(path, ".json") {
            Some(name) => name,
            None => return Ok(None),
        };

        let contents = vfs.read(path)?;

        let value: serde_json::Value = serde_json::from_slice(&contents)
            .map_err(|err| SnapshotError::malformed_json(err, path))?;

        let as_lua = json_to_lua(value).to_string();

        let properties = hashmap! {
            "Value".to_owned() => RbxValue::String {
                value: as_lua,
            },
        };

        let meta_path = path.with_file_name(format!("{}.meta.json", instance_name));

        let mut snapshot = InstanceSnapshot::new()
            .name(instance_name)
            .class_name("ModuleScript")
            .properties(properties)
            .metadata(
                InstanceMetadata::new()
                    .instigating_source(path)
                    .relevant_paths(vec![path.to_path_buf(), meta_path.clone()])
                    .context(context),
            );

        if let Some(meta_contents) = vfs.read(&meta_path).with_not_found()? {
            let mut metadata = AdjacentMetadata::from_slice(&meta_contents, &meta_path)?;
            metadata.apply_all(&mut snapshot);
        }

        Ok(Some(snapshot))
    }
}

fn json_to_lua(value: serde_json::Value) -> Statement {
    unimplemented!()
}

#[cfg(test)]
mod test {
    use super::*;

    use memofs::{InMemoryFs, VfsSnapshot};

    #[test]
    fn instance_from_vfs() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(
            "/foo.json",
            VfsSnapshot::file(r#"{ "x": 5, "y": "hello", "z": [1, 2, 3], "w": null }"#),
        )
        .unwrap();

        let mut vfs = Vfs::new(imfs.clone());

        let instance_snapshot = SnapshotJson::from_vfs(
            &InstanceContext::default(),
            &mut vfs,
            Path::new("/foo.json"),
        )
        .unwrap()
        .unwrap();

        insta::assert_yaml_snapshot!(instance_snapshot);
    }
}
