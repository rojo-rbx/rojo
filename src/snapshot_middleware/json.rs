use std::path::Path;

use maplit::hashmap;
use memofs::{IoResultExt, Vfs};
use rbx_dom_weak::RbxValue;

use crate::{
    lua_ast::{Expression, Statement},
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

        // FIXME: This middleware should not need to know about the .meta.json
        // middleware. Should there be a way to signal "I'm not returning an
        // instance and no one should"?
        if match_file_name(path, ".meta.json").is_some() {
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
            "Source".to_owned() => RbxValue::String {
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
    Statement::Return(json_to_lua_value(value))
}

fn json_to_lua_value(value: serde_json::Value) -> Expression {
    use serde_json::Value;

    match value {
        Value::Null => Expression::Nil,
        Value::Bool(value) => Expression::Bool(value),
        Value::Number(value) => Expression::Number(value.as_f64().unwrap()),
        Value::String(value) => Expression::String(value),
        Value::Array(values) => {
            Expression::Array(values.into_iter().map(json_to_lua_value).collect())
        }
        Value::Object(values) => Expression::table(
            values
                .into_iter()
                .map(|(key, value)| (key.into(), json_to_lua_value(value)))
                .collect(),
        ),
    }
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
            VfsSnapshot::file(
                r#"{
                  "array": [1, 2, 3],
                  "object": {
                    "hello": "world"
                  },
                  "true": true,
                  "false": false,
                  "null": null,
                  "int": 1234,
                  "float": 1234.5452,
                  "1invalidident": "nice"
                }"#,
            ),
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
