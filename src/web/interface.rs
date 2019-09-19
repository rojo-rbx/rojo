//! Defines all the structs needed to interact with the Rojo Serve API.

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

use rbx_dom_weak::{RbxId, RbxValue};
use serde::{Deserialize, Serialize};

use crate::{session_id::SessionId, snapshot::InstanceWithMeta};

/// Server version to report over the API, not exposed outside this crate.
pub(crate) const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Current protocol version, which is required to match.
pub const PROTOCOL_VERSION: u64 = 3;

// TODO
pub type SubscribeMessage = ();

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceMetadata {
    pub ignore_unknown_instances: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Instance<'a> {
    pub name: Cow<'a, str>,
    pub class_name: Cow<'a, str>,
    pub properties: Cow<'a, HashMap<String, RbxValue>>,
    pub children: Cow<'a, [RbxId]>,
    pub metadata: Option<InstanceMetadata>,
}

impl<'a> Instance<'a> {
    pub fn from_rojo_instance<'b>(source: InstanceWithMeta<'b>) -> Instance<'b> {
        Instance {
            name: Cow::Borrowed(source.name()),
            class_name: Cow::Borrowed(source.class_name()),
            properties: Cow::Borrowed(source.properties()),
            children: Cow::Borrowed(source.children()),
            metadata: Some(InstanceMetadata {
                ignore_unknown_instances: source.metadata().ignore_unknown_instances,
            }),
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

/// Response body from /api/subscribe/{cursor}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeResponse {
    pub session_id: SessionId,
    pub message_cursor: u32,
    pub messages: Vec<SubscribeMessage>,
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
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ErrorResponseKind {
    NotFound,
}
