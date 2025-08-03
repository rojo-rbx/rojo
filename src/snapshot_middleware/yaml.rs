use std::path::Path;

use anyhow::Context as _;
use memofs::{IoResultExt, Vfs};
use rbx_dom_weak::ustr;
use yaml_rust2::{Yaml, YamlLoader};

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
    let contents = vfs.read_to_string(path)?;

    let mut values = YamlLoader::load_from_str(&contents)?;
    let value = values
        .pop()
        .context("all YAML documents must contain a document")?;
    if !values.is_empty() {
        anyhow::bail!("Rojo does not currently support multiple documents in a YAML file")
    }

    let as_lua = Statement::Return(yaml_to_luau(value)?);

    let meta_path = path.with_file_name(format!("{}.meta.json", name));

    let mut snapshot = InstanceSnapshot::new()
        .name(name)
        .class_name("ModuleScript")
        .property(ustr("Source"), as_lua.to_string())
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

fn yaml_to_luau(value: Yaml) -> anyhow::Result<Expression> {
    const MAX_FLOAT_INT: i64 = 1 << 53;

    Ok(match value {
        Yaml::String(str) => Expression::String(str),
        Yaml::Boolean(bool) => Expression::Bool(bool),
        Yaml::Integer(int) => {
            if int <= MAX_FLOAT_INT {
                Expression::Number(int as f64)
            } else {
                anyhow::bail!(
                    "the integer '{int}' cannot be losslessly converted into a Luau number"
                )
            }
        }
        Yaml::Real(_) => {
            let value = value.as_f64().expect("value should be a valid f64");
            Expression::Number(value)
        }
        Yaml::Null => Expression::Nil,
        Yaml::Array(values) => {
            let new_values: anyhow::Result<Vec<Expression>> =
                values.into_iter().map(yaml_to_luau).collect();
            Expression::Array(new_values?)
        }
        Yaml::Hash(map) => {
            let new_values: anyhow::Result<Vec<(Expression, Expression)>> = map
                .into_iter()
                .map(|(k, v)| {
                    let k = yaml_to_luau(k)?;
                    let v = yaml_to_luau(v)?;
                    Ok((k, v))
                })
                .collect();
            Expression::table(new_values?)
        }
        Yaml::Alias(_) => {
            anyhow::bail!("Rojo cannot convert YAML aliases to Luau")
        }
        Yaml::BadValue => {
            anyhow::bail!("Rojo cannot convert YAML to Luau because of a parsing error")
        }
    })
}

#[cfg(test)]
mod test {
    use super::*;

    use memofs::{InMemoryFs, VfsSnapshot};
    use rbx_dom_weak::types::Variant;

    #[test]
    fn instance_from_vfs() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(
            "/foo.yaml",
            VfsSnapshot::file(
                r#"
---
string: this is a string
boolean: true
integer: 1337
float: 123456789.5
value-with-hypen: it sure is
sequence:
  - wow
  - 8675309
map:
  key: value
  key2: "value 2"
  key3: 'value 3'
nested-map:
  - key: value
  - key2: "value 2"
  - key3: 'value 3'
whatever_this_is: [i imagine, it's, a, sequence?]
null1: ~
null2: null"#,
            ),
        )
        .unwrap();

        let vfs = Vfs::new(imfs.clone());

        let instance_snapshot = snapshot_yaml(
            &InstanceContext::default(),
            &vfs,
            Path::new("/foo.yaml"),
            "foo",
        )
        .unwrap()
        .unwrap();

        insta::assert_yaml_snapshot!(instance_snapshot);

        let source = instance_snapshot
            .properties
            .get(&ustr("Source"))
            .expect("the result from snapshot_yaml should have a Source property");
        if let Variant::String(source) = source {
            insta::assert_snapshot!(source)
        } else {
            panic!("the Source property from snapshot_yaml was not a String")
        }
    }

    #[test]
    #[should_panic(expected = "multiple documents")]
    fn multiple_documents() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(
            "/foo.yaml",
            VfsSnapshot::file(
                r#"
---
document-1: this is a document
---
document-2: this is also a document"#,
            ),
        )
        .unwrap();

        let vfs = Vfs::new(imfs.clone());

        snapshot_yaml(
            &InstanceContext::default(),
            &vfs,
            Path::new("/foo.yaml"),
            "foo",
        )
        .unwrap()
        .unwrap();
    }

    #[test]
    #[should_panic = "cannot be losslessly converted into a Luau number"]
    fn integer_border() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(
            "/allowed.yaml",
            VfsSnapshot::file(
                r#"
value: 9007199254740992
"#,
            ),
        )
        .unwrap();
        imfs.load_snapshot(
            "/not-allowed.yaml",
            VfsSnapshot::file(
                r#"
value: 9007199254740993
"#,
            ),
        )
        .unwrap();

        let vfs = Vfs::new(imfs.clone());

        assert!(
            snapshot_yaml(
                &InstanceContext::default(),
                &vfs,
                Path::new("/allowed.yaml"),
                "allowed",
            )
            .is_ok(),
            "snapshot_yaml failed to snapshot document with integer '9007199254740992' in it"
        );

        snapshot_yaml(
            &InstanceContext::default(),
            &vfs,
            Path::new("/not-allowed.yaml"),
            "not-allowed",
        )
        .unwrap()
        .unwrap();
    }
}
