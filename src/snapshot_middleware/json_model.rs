use std::{borrow::Cow, collections::HashMap, path::Path, str};

use anyhow::Context;
use memofs::Vfs;
use serde::Deserialize;

use crate::{
    resolution::UnresolvedValue,
    snapshot::{InstanceContext, InstanceSnapshot},
};

use super::util::PathExt;

pub fn snapshot_json_model(
    context: &InstanceContext,
    vfs: &Vfs,
    path: &Path,
) -> anyhow::Result<Option<InstanceSnapshot>> {
    let name = path.file_name_trim_end(".model.json")?;

    let contents = vfs.read(path)?;
    let contents_str = str::from_utf8(&contents)
        .with_context(|| format!("File was not valid UTF-8: {}", path.display()))?;

    if contents_str.trim().is_empty() {
        return Ok(None);
    }

    let instance: JsonModel = serde_json::from_str(contents_str)
        .with_context(|| format!("File is not a valid JSON model: {}", path.display()))?;

    let mut snapshot = instance
        .core
        .into_snapshot(name.to_owned())
        .with_context(|| format!("Could not load JSON model: {}", path.display()))?;

    snapshot.metadata = snapshot
        .metadata
        .instigating_source(path)
        .relevant_paths(vec![path.to_path_buf()])
        .context(context);

    Ok(Some(snapshot))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct JsonModel {
    name: Option<String>,

    #[serde(flatten)]
    core: JsonModelCore,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct JsonModelInstance {
    name: String,

    #[serde(flatten)]
    core: JsonModelCore,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct JsonModelCore {
    class_name: String,

    #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
    children: Vec<JsonModelInstance>,

    #[serde(default = "HashMap::new", skip_serializing_if = "HashMap::is_empty")]
    properties: HashMap<String, UnresolvedValue>,
}

impl JsonModelCore {
    fn into_snapshot(self, name: String) -> anyhow::Result<InstanceSnapshot> {
        let class_name = self.class_name;

        let mut children = Vec::with_capacity(self.children.len());
        for child in self.children {
            children.push(child.core.into_snapshot(child.name)?);
        }

        let mut properties = HashMap::with_capacity(self.properties.len());
        for (key, unresolved) in self.properties {
            let value = unresolved.resolve(&class_name, &key)?;
            properties.insert(key, value);
        }

        Ok(InstanceSnapshot {
            snapshot_id: None,
            metadata: Default::default(),
            name: Cow::Owned(name),
            class_name: Cow::Owned(class_name),
            properties,
            children,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use memofs::{InMemoryFs, VfsSnapshot};

    #[test]
    fn model_from_vfs() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(
            "/foo.model.json",
            VfsSnapshot::file(
                r#"
                    {
                      "Name": "children",
                      "ClassName": "IntValue",
                      "Properties": {
                        "Value": 5
                      },
                      "Children": [
                        {
                          "Name": "The Child",
                          "ClassName": "StringValue"
                        }
                      ]
                    }
                "#,
            ),
        )
        .unwrap();

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_json_model(
            &InstanceContext::default(),
            &mut vfs,
            Path::new("/foo.model.json"),
        )
        .unwrap()
        .unwrap();

        insta::assert_yaml_snapshot!(instance_snapshot);
    }
}
