use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
    path::Path,
    str,
};

use anyhow::Context;
use memofs::Vfs;
use rbx_dom_weak::types::{Attributes, Ref, Variant};
use serde::{Deserialize, Serialize};

use crate::{
    resolution::UnresolvedValue,
    snapshot::{InstanceContext, InstanceSnapshot},
    syncback::{filter_properties_preallocated, FsSnapshot, SyncbackReturn, SyncbackSnapshot},
    RojoRef,
};

pub fn snapshot_json_model(
    context: &InstanceContext,
    vfs: &Vfs,
    path: &Path,
    name: &str,
) -> anyhow::Result<Option<InstanceSnapshot>> {
    let contents = vfs.read(path)?;
    let contents_str = str::from_utf8(&contents)
        .with_context(|| format!("File was not valid UTF-8: {}", path.display()))?;

    if contents_str.trim().is_empty() {
        return Ok(None);
    }

    let mut instance: JsonModel = serde_json::from_str(contents_str)
        .with_context(|| format!("File is not a valid JSON model: {}", path.display()))?;

    if let Some(top_level_name) = &instance.name {
        let new_name = format!("{}.model.json", top_level_name);

        log::warn!(
            "Model at path {} had a top-level Name field. \
            This field has been ignored since Rojo 6.0.\n\
            Consider removing this field and renaming the file to {}.",
            new_name,
            path.display()
        );
    }

    instance.name = Some(name.to_owned());

    let id = instance.id.take().map(RojoRef::new);

    let mut snapshot = instance
        .into_snapshot()
        .with_context(|| format!("Could not load JSON model: {}", path.display()))?;

    snapshot.metadata = snapshot
        .metadata
        .instigating_source(path)
        .relevant_paths(vec![path.to_path_buf()])
        .context(context)
        .specified_id(id);

    Ok(Some(snapshot))
}

pub fn syncback_json_model<'sync>(
    snapshot: &SyncbackSnapshot<'sync>,
) -> anyhow::Result<SyncbackReturn<'sync>> {
    let mut property_buffer = Vec::with_capacity(snapshot.new_inst().properties.len());

    let mut model = json_model_from_pair(snapshot, &mut property_buffer, snapshot.new);
    // We don't need the name on the root, but we do for children.
    model.name = None;

    Ok(SyncbackReturn {
        fs_snapshot: FsSnapshot::new().with_added_file(
            &snapshot.path,
            serde_json::to_vec_pretty(&model).context("failed to serialize new JSON Model")?,
        ),
        children: Vec::new(),
        removed_children: Vec::new(),
    })
}

fn json_model_from_pair<'sync>(
    snapshot: &SyncbackSnapshot<'sync>,
    prop_buffer: &mut Vec<(&'sync str, &'sync Variant)>,
    new: Ref,
) -> JsonModel {
    let new_inst = snapshot
        .get_new_instance(new)
        .expect("all new referents passed to json_model_from_pair should exist");

    filter_properties_preallocated(snapshot.project(), new_inst, prop_buffer);

    let mut properties = BTreeMap::new();
    let mut attributes = BTreeMap::new();
    for (name, value) in prop_buffer.drain(..) {
        match value {
            Variant::Attributes(attrs) => {
                for (attr_name, attr_value) in attrs.iter() {
                    attributes.insert(
                        attr_name.clone(),
                        UnresolvedValue::from_variant_unambiguous(attr_value.clone()),
                    );
                }
            }
            Variant::SharedString(_) => {
                log::warn!(
                "Rojo cannot serialize the property {}.{name} in model.json files.\n\
                If this is not acceptable, resave the Instance at '{}' manually as an RBXM or RBXMX.", new_inst.class, snapshot.get_new_inst_path(new))
            }
            _ => {
                properties.insert(
                    name.to_owned(),
                    UnresolvedValue::from_variant(value.clone(), &new_inst.class, name),
                );
            }
        }
    }

    let mut children = Vec::with_capacity(new_inst.children().len());

    for new_child_ref in new_inst.children() {
        children.push(json_model_from_pair(snapshot, prop_buffer, *new_child_ref))
    }

    JsonModel {
        name: Some(new_inst.name.clone()),
        class_name: new_inst.class.clone(),
        children,
        properties,
        attributes,
        id: None,
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonModel {
    #[serde(alias = "Name", skip_serializing_if = "Option::is_none")]
    name: Option<String>,

    #[serde(alias = "ClassName")]
    class_name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,

    #[serde(
        alias = "Children",
        default = "Vec::new",
        skip_serializing_if = "Vec::is_empty"
    )]
    children: Vec<JsonModel>,

    #[serde(
        alias = "Properties",
        default = "BTreeMap::new",
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    properties: BTreeMap<String, UnresolvedValue>,

    #[serde(default = "BTreeMap::new", skip_serializing_if = "BTreeMap::is_empty")]
    attributes: BTreeMap<String, UnresolvedValue>,
}

impl JsonModel {
    fn into_snapshot(self) -> anyhow::Result<InstanceSnapshot> {
        let name = self.name.unwrap_or_else(|| self.class_name.clone());
        let class_name = self.class_name;

        let mut children = Vec::with_capacity(self.children.len());
        for child in self.children {
            children.push(child.into_snapshot()?);
        }

        let mut properties = HashMap::with_capacity(self.properties.len());
        for (key, unresolved) in self.properties {
            let value = unresolved.resolve(&class_name, &key)?;
            properties.insert(key, value);
        }

        if !self.attributes.is_empty() {
            let mut attributes = Attributes::new();

            for (key, unresolved) in self.attributes {
                let value = unresolved.resolve_unambiguous()?;
                attributes.insert(key, value);
            }

            properties.insert("Attributes".into(), attributes.into());
        }

        Ok(InstanceSnapshot {
            snapshot_id: Ref::none(),
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
                      "className": "IntValue",
                      "properties": {
                        "Value": 5
                      },
                      "children": [
                        {
                          "name": "The Child",
                          "className": "StringValue"
                        }
                      ]
                    }
                "#,
            ),
        )
        .unwrap();

        let vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_json_model(
            &InstanceContext::default(),
            &vfs,
            Path::new("/foo.model.json"),
            "foo",
        )
        .unwrap()
        .unwrap();

        insta::assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn model_from_vfs_legacy() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(
            "/foo.model.json",
            VfsSnapshot::file(
                r#"
                    {
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

        let vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_json_model(
            &InstanceContext::default(),
            &vfs,
            Path::new("/foo.model.json"),
            "foo",
        )
        .unwrap()
        .unwrap();

        insta::assert_yaml_snapshot!(instance_snapshot);
    }
}
