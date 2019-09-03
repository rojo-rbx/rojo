//! Defines all the structs needed to interact with the Rojo API from an
//! automation test perspective.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::session_id::SessionId;

pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const PROTOCOL_VERSION: u64 = 3;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfoResponse<'a> {
    pub session_id: SessionId,
    pub server_version: &'a str,
    pub protocol_version: u64,
    pub expected_place_ids: Option<HashSet<u64>>,
    // pub root_instance_id: RbxId,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadResponse {
    pub session_id: SessionId,
    // pub message_cursor: u32,
    // pub instances: HashMap<RbxId, InstanceWithMetadata<'a>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeResponse {
    pub session_id: SessionId,
    // pub message_cursor: u32,
    // pub messages: Cow<'a, [InstanceChanges]>,
}
