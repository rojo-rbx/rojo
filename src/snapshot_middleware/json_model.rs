use std::{borrow::Cow, collections::HashMap};

use rbx_dom_weak::{RbxId, RbxTree, UnresolvedRbxValue};
use rbx_reflection::try_resolve_value;
use serde::Deserialize;

use crate::{
    imfs::{Imfs, ImfsEntry, ImfsFetcher},
    snapshot::InstanceSnapshot,
};

use super::middleware::{SnapshotFileResult, SnapshotInstanceResult, SnapshotMiddleware};

pub struct SnapshotJsonModel;

impl SnapshotMiddleware for SnapshotJsonModel {
    fn from_imfs<F: ImfsFetcher>(
        imfs: &mut Imfs<F>,
        entry: &ImfsEntry,
    ) -> SnapshotInstanceResult<'static> {
        if entry.is_directory() {
            return Ok(None);
        }

        let file_name = entry.path().file_name().unwrap().to_string_lossy();

        let instance_name = match match_trailing(&file_name, ".model.json") {
            Some(name) => name.to_owned(),
            None => return Ok(None),
        };

        let instance: JsonModel =
            serde_json::from_slice(entry.contents(imfs)?).expect("TODO: Handle serde_json errors");

        if let Some(json_name) = &instance.name {
            if json_name != &instance_name {
                log::warn!(
                    "Name from JSON model did not match its file name: {}",
                    entry.path().display()
                );
                log::warn!(
                    "In Rojo <  alpha 14, this model is named \"{}\" (from its 'Name' property)",
                    json_name
                );
                log::warn!(
                    "In Rojo >= alpha 14, this model is named \"{}\" (from its file name)",
                    instance_name
                );
                log::warn!("'Name' for the top-level instance in a JSON model is now optional and will be ignored.");
            }
        }

        let mut snapshot = instance.core.into_snapshot(instance_name);

        snapshot.metadata.contributing_paths = vec![entry.path().to_path_buf().into()];

        Ok(Some(snapshot))
    }

    fn from_instance(_tree: &RbxTree, _id: RbxId) -> SnapshotFileResult {
        unimplemented!("Snapshotting models");
    }
}

fn match_trailing<'a>(input: &'a str, trailer: &str) -> Option<&'a str> {
    if input.ends_with(trailer) {
        let end = input.len().saturating_sub(trailer.len());
        Some(&input[..end])
    } else {
        None
    }
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
    properties: HashMap<String, UnresolvedRbxValue>,
}

impl JsonModelCore {
    fn into_snapshot(self, name: String) -> InstanceSnapshot<'static> {
        let class_name = self.class_name;

        let children = self
            .children
            .into_iter()
            .map(|child| child.core.into_snapshot(child.name))
            .collect();

        let properties = self
            .properties
            .into_iter()
            .map(|(key, value)| {
                try_resolve_value(&class_name, &key, &value).map(|resolved| (key, resolved))
            })
            .collect::<Result<HashMap<_, _>, _>>()
            .expect("TODO: Handle rbx_reflection errors");

        InstanceSnapshot {
            snapshot_id: None,
            metadata: Default::default(), // TODO: Allow the user to specify metadata in the JSON model again.
            name: Cow::Owned(name),
            class_name: Cow::Owned(class_name),
            properties,
            children,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use insta::assert_yaml_snapshot;
    use maplit::hashmap;
    use rbx_dom_weak::RbxValue;

    use crate::imfs::{ImfsDebug, ImfsSnapshot, NoopFetcher};

    #[test]
    fn model_from_imfs() {
        let mut imfs = Imfs::new(NoopFetcher);
        let file = ImfsSnapshot::file(
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
        );

        imfs.debug_load_snapshot("/foo.model.json", file);

        let entry = imfs.get("/foo.model.json").unwrap();
        let instance_snapshot = SnapshotJsonModel::from_imfs(&mut imfs, &entry)
            .unwrap()
            .unwrap();

        assert_yaml_snapshot!(instance_snapshot);
    }
}
