use std::{borrow::Cow, collections::HashMap};

use rbx_dom_weak::UnresolvedRbxValue;
use rbx_reflection::try_resolve_value;
use serde::{Deserialize, Serialize};

use crate::snapshot::InstanceSnapshot;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdjacentMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_unknown_instances: Option<bool>,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub properties: HashMap<String, UnresolvedRbxValue>,
}

impl AdjacentMetadata {
    pub fn from_slice(slice: &[u8]) -> Self {
        serde_json::from_slice(slice)
            // TODO: Turn into error type
            .expect(".meta.json file was malformed")
    }

    pub fn apply_ignore_unknown_instances(&mut self, snapshot: &mut InstanceSnapshot) {
        if let Some(ignore) = self.ignore_unknown_instances.take() {
            snapshot.metadata.ignore_unknown_instances = ignore;
        }
    }

    pub fn apply_class_name(&mut self, snapshot: &mut InstanceSnapshot) {
        if let Some(class_name) = self.class_name.take() {
            snapshot.class_name = Cow::Owned(class_name);
        }
    }

    pub fn apply_properties(&mut self, snapshot: &mut InstanceSnapshot) {
        let class_name = &snapshot.class_name;

        let source_properties = self.properties.drain().map(|(key, value)| {
            try_resolve_value(class_name, &key, &value)
                .map(|resolved| (key, resolved))
                .expect("TODO: Handle rbx_reflection errors")
        });

        for (key, value) in source_properties {
            snapshot.properties.insert(key, value);
        }
    }

    pub fn apply_all(&mut self, snapshot: &mut InstanceSnapshot) {
        self.apply_ignore_unknown_instances(snapshot);
        self.apply_class_name(snapshot);
        self.apply_properties(snapshot);
    }
}
