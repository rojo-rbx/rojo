//! Defines all the structs needed to interact with the Rojo Serve API. This is
//! useful for tests to be able to use the same data structures as the
//! implementation.

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

use rbx_dom_weak::{
    types::{Ref, Variant, VariantType},
    Ustr, UstrMap,
};
use serde::{Deserialize, Serialize};

use crate::{
    session_id::SessionId,
    snapshot::{
        AppliedPatchSet, InstanceMetadata as RojoInstanceMetadata, InstanceWithMeta, RojoTree,
    },
};

/// Server version to report over the API, not exposed outside this crate.
pub(crate) const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Current protocol version, which is required to match.
pub const PROTOCOL_VERSION: u64 = 4;

/// Message returned by Rojo API when a change has occurred.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeMessage<'a> {
    pub removed: Vec<Ref>,
    pub added: HashMap<Ref, Instance<'a>>,
    pub updated: Vec<InstanceUpdate>,
}

impl<'a> SubscribeMessage<'a> {
    pub(crate) fn from_patch_update(tree: &'a RojoTree, patch: AppliedPatchSet) -> Self {
        let removed = patch.removed;

        let mut added = HashMap::new();
        for id in patch.added {
            let instance = tree.get_instance(id).unwrap();
            added.insert(id, Instance::from_rojo_instance(instance));

            for instance in tree.descendants(id) {
                added.insert(instance.id(), Instance::from_rojo_instance(instance));
            }
        }

        let updated = patch
            .updated
            .into_iter()
            .map(|update| {
                let changed_metadata = update
                    .changed_metadata
                    .as_ref()
                    .map(InstanceMetadata::from_rojo_metadata);

                let changed_properties = update
                    .changed_properties
                    .into_iter()
                    .filter(|(_key, value)| property_filter(value.as_ref()))
                    .collect();

                InstanceUpdate {
                    id: update.id,
                    changed_name: update.changed_name,
                    changed_class_name: update.changed_class_name,
                    changed_properties,
                    changed_metadata,
                }
            })
            .collect();

        Self {
            removed,
            added,
            updated,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceUpdate {
    pub id: Ref,
    pub changed_name: Option<String>,
    pub changed_class_name: Option<Ustr>,

    // TODO: Transform from UstrMap<String, Option<_>> to something else, since
    // null will get lost when decoding from JSON in some languages.
    #[serde(default)]
    pub changed_properties: UstrMap<Option<Variant>>,
    pub changed_metadata: Option<InstanceMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceMetadata {
    pub ignore_unknown_instances: bool,
}

impl InstanceMetadata {
    pub(crate) fn from_rojo_metadata(meta: &RojoInstanceMetadata) -> Self {
        Self {
            ignore_unknown_instances: meta.ignore_unknown_instances,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Instance<'a> {
    pub id: Ref,
    pub parent: Ref,
    pub name: Cow<'a, str>,
    pub class_name: Ustr,
    pub properties: UstrMap<Cow<'a, Variant>>,
    pub children: Cow<'a, [Ref]>,
    pub metadata: Option<InstanceMetadata>,
}

impl Instance<'_> {
    pub(crate) fn from_rojo_instance(source: InstanceWithMeta<'_>) -> Instance<'_> {
        let properties = source
            .properties()
            .iter()
            .filter(|(_key, value)| property_filter(Some(value)))
            .map(|(key, value)| (*key, Cow::Borrowed(value)))
            .collect();

        Instance {
            id: source.id(),
            parent: source.parent(),
            name: Cow::Borrowed(source.name()),
            class_name: source.class_name(),
            properties,
            children: Cow::Borrowed(source.children()),
            metadata: Some(InstanceMetadata::from_rojo_metadata(source.metadata())),
        }
    }
}

fn property_filter(value: Option<&Variant>) -> bool {
    let ty = value.map(|value| value.ty());

    // Lua can't do anything with SharedString values. They also can't be
    // serialized directly by Serde!
    ty != Some(VariantType::SharedString)
}

/// Response body from /api/rojo
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfoResponse {
    pub session_id: SessionId,
    pub server_version: String,
    pub protocol_version: u64,
    pub project_name: String,
    pub expected_place_ids: Option<HashSet<u64>>,
    pub unexpected_place_ids: Option<HashSet<u64>>,
    pub game_id: Option<u64>,
    pub place_id: Option<u64>,
    pub root_instance_id: Ref,
}

/// Response body from /api/read/{id}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadResponse<'a> {
    pub session_id: SessionId,
    pub message_cursor: u32,
    pub instances: HashMap<Ref, Instance<'a>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteRequest {
    pub session_id: SessionId,
    pub removed: Vec<Ref>,

    #[serde(default)]
    pub added: HashMap<Ref, ()>,
    pub updated: Vec<InstanceUpdate>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteResponse {
    pub session_id: SessionId,
}

/// Response body from /api/subscribe/{cursor}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeResponse<'a> {
    pub session_id: SessionId,
    pub message_cursor: u32,
    pub messages: Vec<SubscribeMessage<'a>>,
}

/// Response body from /api/open/{id}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenResponse {
    pub session_id: SessionId,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializeResponse {
    pub session_id: SessionId,
    pub model_contents: BufferEncode,
}

/// Using this struct we can force Roblox to JSONDecode this as a buffer.
/// This is what Roblox's serde APIs use, so it saves a step in the plugin.
#[derive(Debug, Serialize, Deserialize)]
pub struct BufferEncode {
    m: (),
    t: Cow<'static, str>,
    base64: String,
}

impl BufferEncode {
    pub fn new(content: Vec<u8>) -> Self {
        let base64 = data_encoding::BASE64.encode(&content);
        Self {
            m: (),
            t: Cow::Borrowed("buffer"),
            base64,
        }
    }

    pub fn model(&self) -> &str {
        &self.base64
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefPatchResponse<'a> {
    pub session_id: SessionId,
    pub patch: SubscribeMessage<'a>,
}

/// General response type returned from all Rojo routes
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    kind: ErrorResponseKind,
    details: String,
}

impl ErrorResponse {
    pub fn not_found<S: Into<String>>(details: S) -> Self {
        Self {
            kind: ErrorResponseKind::NotFound,
            details: details.into(),
        }
    }

    pub fn bad_request<S: Into<String>>(details: S) -> Self {
        Self {
            kind: ErrorResponseKind::BadRequest,
            details: details.into(),
        }
    }

    pub fn internal_error<S: Into<String>>(details: S) -> Self {
        Self {
            kind: ErrorResponseKind::InternalError,
            details: details.into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ErrorResponseKind {
    NotFound,
    BadRequest,
    InternalError,
}
