use std::path::Path;

use anyhow::Context;
use memofs::Vfs;
use rbx_dom_weak::ustr;

use crate::{
    lua_ast::{Expression, Statement},
    snapshot::{InstanceContext, InstanceMetadata, InstanceSnapshot},
};

use super::meta_file::AdjacentMetadata;

pub fn snapshot_toml(
    context: &InstanceContext,
    vfs: &Vfs,
    path: &Path,
    name: &str,
) -> anyhow::Result<Option<InstanceSnapshot>> {
    let contents = vfs.read(path)?;

    let value: toml::Value = toml::from_slice(&contents)
        .with_context(|| format!("File contains malformed TOML: {}", path.display()))?;

    let as_lua = toml_to_lua(value).to_string();

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

fn toml_to_lua(value: toml::Value) -> Statement {
    Statement::Return(toml_to_lua_value(value))
}

fn toml_to_lua_value(value: toml::Value) -> Expression {
    use toml::Value;

    match value {
        Value::Datetime(value) => Expression::String(value.to_string()),
        Value::Boolean(value) => Expression::Bool(value),
        Value::Float(value) => Expression::Number(value),
        Value::Integer(value) => Expression::Number(value as f64),
        Value::String(value) => Expression::String(value),
        Value::Array(values) => {
            Expression::Array(values.into_iter().map(toml_to_lua_value).collect())
        }
        Value::Table(values) => Expression::table(
            values
                .into_iter()
                .map(|(key, value)| (key.into(), toml_to_lua_value(value)))
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
            "/foo.toml",
            VfsSnapshot::file(
                r#"
                  array = [1, 2, 3]
                  true = true
                  false = false
                  int = 1234
                  float = 1234.5452
                  "1invalidident" = "nice"

                  [object]
                  hello = "world"

                  [dates]
                  offset1 = 1979-05-27T00:32:00.999999-07:00
                  offset2 = 1979-05-27 07:32:00Z
                  localdatetime = 1979-05-27T07:32:00
                  localdate = 1979-05-27
                  localtime = 00:32:00.999999
                "#,
            ),
        )
        .unwrap();

        let vfs = Vfs::new(imfs.clone());

        let instance_snapshot = snapshot_toml(
            &InstanceContext::default(),
            &vfs,
            Path::new("/foo.toml"),
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
            "foo.toml",
            VfsSnapshot::file(
                r#"
                  array = [1, 2, 3]
                  int = 123
                  bool = false
                "#,
            ),
        )
        .unwrap();
        imfs.load_snapshot(
            "foo.meta.json",
            VfsSnapshot::file(
                r#"{
                    "id": "manually specified"
                }"#,
            ),
        )
        .unwrap();

        let vfs = Vfs::new(imfs.clone());

        let instance_snapshot = snapshot_toml(
            &InstanceContext::default(),
            &vfs,
            Path::new("foo.toml"),
            "foo",
        )
        .unwrap()
        .unwrap();

        insta::assert_yaml_snapshot!(instance_snapshot);
    }
}
