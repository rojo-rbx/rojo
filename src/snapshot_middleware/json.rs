use std::path::Path;

use maplit::hashmap;
use memofs::{IoResultExt, Vfs};
use rbx_dom_weak::RbxValue;

use crate::{
    lua_ast::{Expression, Statement},
    snapshot::{InstanceContext, InstanceMetadata, InstanceSnapshot},
};

use super::{
    error::SnapshotError, meta_file::AdjacentMetadata, middleware::SnapshotInstanceResult,
};

pub fn snapshot_json(
    context: &InstanceContext,
    vfs: &Vfs,
    path: &Path,
    instance_name: &str,
) -> SnapshotInstanceResult {
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

        let instance_snapshot = snapshot_json(
            &InstanceContext::default(),
            &mut vfs,
            Path::new("/foo.json"),
            "foo",
        )
        .unwrap()
        .unwrap();

        insta::assert_yaml_snapshot!(instance_snapshot);
    }
}
