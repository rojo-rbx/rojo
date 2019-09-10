//! Defines all the structs needed to interact with the Rojo Serve API.

use std::collections::{HashMap, HashSet};

use rbx_dom_weak::RbxId;
use serde::{Deserialize, Serialize};

use crate::session_id::SessionId;

/// Server version to report over the API, not exposed outside this crate.
pub(crate) const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Current protocol version, which is required to match.
pub const PROTOCOL_VERSION: u64 = 3;

// TODO
#[derive(Debug, Serialize, Deserialize)]
pub struct SubscribeMessage;

// TODO
#[derive(Debug, Serialize, Deserialize)]
pub struct Instance;

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
pub struct ReadResponse {
    pub session_id: SessionId,
    pub message_cursor: u32,
    pub instances: HashMap<RbxId, Instance>,
}

/// Response body from /api/subscribe/{cursor}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeResponse {
    pub session_id: SessionId,
    pub message_cursor: u32,
    pub messages: Vec<SubscribeMessage>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NotFoundError;
