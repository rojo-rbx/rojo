use std::path::Path;

use anyhow::Context;
use maplit::hashmap;
use memofs::{IoResultExt, Vfs};

use crate::{
    lua_ast::{Expression, Statement},
    snapshot::{InstanceContext, InstanceMetadata, InstanceSnapshot},
};

use super::meta_file::AdjacentMetadata;

pub fn snapshot_yaml(
    context: &InstanceContext,
    vfs: &Vfs,
    path: &Path,
    name: &str,
) -> anyhow::Result<Option<InstanceSnapshot>> {
    let contents = vfs.read(path)?;

    let value: serde_yaml::Value = serde_yaml::from_slice(&contents)
        .with_context(|| format!("File contains malformed YAML: {}", path.display()))?;

    let as_lua = yaml_to_lua(value).to_string();

    let properties = hashmap! {
        "Source".to_owned() => as_lua.into(),
    };

    let meta_path = path.with_file_name(format!("{}.meta.json", name));

    let mut snapshot = InstanceSnapshot::new()
        .name(name)
        .class_name("ModuleScript")
        .properties(properties)
        .metadata(
            InstanceMetadata::new()
                .instigating_source(path)
                .relevant_paths(vec![path.to_path_buf(), meta_path.clone()])
                .context(context),
        );

    if let Some(meta_contents) = vfs.read(&meta_path).with_not_found()? {
        let mut metadata = AdjacentMetadata::from_slice(&meta_contents, meta_path)?;
        metadata.apply_all(&mut snapshot)?;
    }

    Ok(Some(snapshot))
}

fn yaml_to_lua(value: serde_yaml::Value) -> Statement {
    Statement::Return(yaml_to_lua_value(value))
}

fn yaml_to_lua_value(value: serde_yaml::Value) -> Expression {
    use serde_yaml::Value;

    match value {
        Value::Bool(value) => Expression::Bool(value),
        Value::Null => Expression::Nil,
        Value::Number(value) => Expression::Number(value.as_f64().unwrap()),
        Value::String(value) => Expression::String(value),
        Value::Mapping(map) => Expression::table(
            map.into_iter()
                .map(|(key, value)| (yaml_to_lua_value(key), yaml_to_lua_value(value)))
                .collect(),
        ),
        Value::Sequence(seq) => Expression::Array(seq.into_iter().map(yaml_to_lua_value).collect()),
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
            "/foo.yml",
            VfsSnapshot::file(
                r#"
---
string: this is a string
boolean: true
number: 1337
value-with-hypen: it sure is
sequence:
  - wow
  - 8675309
map:
  - key: value
  - key2: "value 2"
  - key3: 'value 3'
whatever_this_is: [i imagine, it's, a, sequence?]"#,
            ),
        )
        .unwrap();

        let vfs = Vfs::new(imfs.clone());

        let instance_snapshot = snapshot_yaml(
            &InstanceContext::default(),
            &vfs,
            Path::new("/foo.yml"),
            "foo",
        )
        .unwrap()
        .unwrap();

        insta::assert_yaml_snapshot!(instance_snapshot);
    }
}
