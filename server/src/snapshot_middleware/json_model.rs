use std::{
    borrow::Cow,
    collections::HashMap,
};

use rbx_reflection::try_resolve_value;
use rbx_dom_weak::{RbxTree, RbxId, UnresolvedRbxValue};
use serde::{Deserialize};

use crate::{
    imfs::new::{Imfs, ImfsFetcher, ImfsEntry},
    snapshot::InstanceSnapshot,
};

use super::{
    middleware::{SnapshotMiddleware, SnapshotInstanceResult, SnapshotFileResult},
};

pub struct SnapshotJsonModel;

impl SnapshotMiddleware for SnapshotJsonModel {
    fn from_imfs<F: ImfsFetcher>(
        imfs: &mut Imfs<F>,
        entry: &ImfsEntry,
    ) -> SnapshotInstanceResult<'static> {
        if entry.is_directory() {
            return Ok(None);
        }

        let file_name = entry.path()
            .file_name().unwrap().to_string_lossy();

        let instance_name = match match_trailing(&file_name, ".model.json") {
            Some(name) => name.to_owned(),
            None => return Ok(None),
        };

        let instance: JsonModelInstance = serde_json::from_slice(entry.contents(imfs)?)
            .expect("TODO: Handle serde_json errors");

        let mut snapshot = instance.into_snapshot();
        snapshot.name = Cow::Owned(instance_name);

        Ok(Some(snapshot))
    }

    fn from_instance(
        _tree: &RbxTree,
        _id: RbxId,
    ) -> SnapshotFileResult {
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
struct JsonModelInstance {
    name: String,
    class_name: String,

    #[serde(default = "Vec::new")]
    children: Vec<JsonModelInstance>,

    #[serde(default = "HashMap::new")]
    properties: HashMap<String, UnresolvedRbxValue>,
}

impl JsonModelInstance {
    fn into_snapshot(self) -> InstanceSnapshot<'static> {
        let class_name = self.class_name;

        let children = self.children.into_iter()
            .map(JsonModelInstance::into_snapshot)
            .collect();

        let properties = self.properties.into_iter()
            .map(|(key, value)| {
                try_resolve_value(&class_name, &key, &value)
                    .map(|resolved| (key, resolved))
            })
            .collect::<Result<HashMap<_, _>, _>>()
            .expect("TODO: Handle rbx_reflection errors");

        InstanceSnapshot {
            snapshot_id: None,
            name: Cow::Owned(self.name),
            class_name: Cow::Owned(class_name),
            properties,
            children,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use maplit::hashmap;
    use rbx_dom_weak::RbxValue;

    use crate::imfs::new::{ImfsSnapshot, NoopFetcher};

    #[test]
    fn model_from_imfs() {
        let mut imfs = Imfs::new(NoopFetcher);
        let file = ImfsSnapshot::file(r#"
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
        "#);

        imfs.load_from_snapshot("/foo.model.json", file);

        let entry = imfs.get("/foo.model.json").unwrap();
        let instance_snapshot = SnapshotJsonModel::from_imfs(&mut imfs, &entry).unwrap().unwrap();

        assert_eq!(instance_snapshot, InstanceSnapshot {
            snapshot_id: None,
            name: Cow::Borrowed("foo"),
            class_name: Cow::Borrowed("IntValue"),
            properties: hashmap! {
                "Value".to_owned() => RbxValue::Int32 {
                    value: 5,
                },
            },
            children: vec![
                InstanceSnapshot {
                    snapshot_id: None,
                    name: Cow::Borrowed("The Child"),
                    class_name: Cow::Borrowed("StringValue"),
                    properties: HashMap::new(),
                    children: Vec::new(),
                },
            ],
        });
    }
}