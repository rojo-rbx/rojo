//! Defines all the structs needed to interact with the Rojo Serve API.

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

use rbx_dom_weak::{RbxId, RbxValue};
use serde::{Deserialize, Serialize};

use crate::{
    session_id::SessionId,
    snapshot::{InstanceMetadata as RojoInstanceMetadata, InstanceWithMeta},
};

/// Server version to report over the API, not exposed outside this crate.
pub(crate) const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Current protocol version, which is required to match.
pub const PROTOCOL_VERSION: u64 = 3;

/// Message returned by Rojo API when a change has occurred.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeMessage<'a> {
    pub removed: Vec<RbxId>,
    pub added: HashMap<RbxId, Instance<'a>>,
    pub updated: Vec<InstanceUpdate>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceUpdate {
    pub id: RbxId,
    pub changed_name: Option<String>,
    pub changed_class_name: Option<String>,

    // TODO: Transform from HashMap<String, Option<_>> to something else, since
    // null will get lost when decoding from JSON in some languages.
    #[serde(default)]
    pub changed_properties: HashMap<String, Option<RbxValue>>,
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
    pub id: RbxId,
    pub parent: Option<RbxId>,
    pub name: Cow<'a, str>,
    pub class_name: Cow<'a, str>,
    pub properties: Cow<'a, HashMap<String, RbxValue>>,
    pub children: Cow<'a, [RbxId]>,
    pub metadata: Option<InstanceMetadata>,
}

impl<'a> Instance<'a> {
    pub(crate) fn from_rojo_instance(source: InstanceWithMeta<'_>) -> Instance<'_> {
        Instance {
            id: source.id(),
            parent: source.parent(),
            name: Cow::Borrowed(source.name()),
            class_name: Cow::Borrowed(source.class_name()),
            properties: Cow::Borrowed(source.properties()),
            children: Cow::Borrowed(source.children()),
            metadata: Some(InstanceMetadata::from_rojo_metadata(source.metadata())),
        }
    }
}

/// Response body from /api/rojo
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfoResponse {
    pub session_id: SessionId,
    pub server_version: String,
    pub protocol_version: u64,
    pub expected_place_ids: Option<HashSet<u64>>,
    pub root_instance_id: RbxId,
}

/// Response body from /api/read/{id}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadResponse<'a> {
    pub session_id: SessionId,
    pub message_cursor: u32,
    pub instances: HashMap<RbxId, Instance<'a>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteRequest {
    pub session_id: SessionId,
    pub removed: Vec<RbxId>,

    #[serde(default)]
    pub added: HashMap<RbxId, ()>,
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
