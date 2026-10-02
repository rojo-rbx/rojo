use std::path::Path;

use memofs::Vfs;
use rbx_dom_weak::ustr;

use crate::{
    json,
    lua_ast::{Expression, Statement},
    snapshot::{InstanceContext, InstanceMetadata, InstanceSnapshot},
};

use super::meta_file::AdjacentMetadata;

pub fn snapshot_json(
    context: &InstanceContext,
    vfs: &Vfs,
    path: &Path,
    name: &str,
) -> anyhow::Result<Option<InstanceSnapshot>> {
    let contents = vfs.read(path)?;

    let value = json::parse_value_from_slice_with_context(&contents, || {
        format!("File contains malformed JSON: {}", path.display())
    })?;

    let as_lua = json_to_lua(value).to_string();

    let mut snapshot = InstanceSnapshot::new()
        .name(name)
        .class_name("ModuleScript")
        .property(ustr("Source"), as_lua)
        .metadata(
            InstanceMetadata::new()
                .instigating_source(path)
                .relevant_paths(vec![vfs.canonicalize(path)?])
                .context(context),
        );

    AdjacentMetadata::read_and_apply_all(vfs, path, name, &mut snapshot)?;

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

        let vfs = Vfs::new(imfs.clone());

        let instance_snapshot = snapshot_json(
            &InstanceContext::default(),
            &vfs,
            Path::new("/foo.json"),
            "foo",
        )
        .unwrap()
        .unwrap();

        insta::assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn with_metadata() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(
            "/foo.json",
            VfsSnapshot::file(
                r#"{
                    "array": [1, 2, 3],
                    "int": 1234,
                    "float": 1234.5452,
                }"#,
            ),
        )
        .unwrap();
        imfs.load_snapshot(
            "/foo.meta.json",
            VfsSnapshot::file(
                r#"{
                    "id": "manually specified"
                }"#,
            ),
        )
        .unwrap();

        let vfs = Vfs::new(imfs.clone());

        let instance_snapshot = snapshot_json(
            &InstanceContext::default(),
            &vfs,
            Path::new("/foo.json"),
            "foo",
        )
        .unwrap()
        .unwrap();

        insta::assert_yaml_snapshot!(instance_snapshot);
    }
}
