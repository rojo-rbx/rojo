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
use strum::Display;

pub use crate::automation::{
    AutomationJobState, AutomationMapEntry, AutomationRequest, AutomationResult, AutomationUdim,
    AutomationValue, AutomationVector2, InspectNode, InspectRequest, InspectResult, InspectTarget,
    InstanceReference,
};
pub use crate::automation_status::{AutomationRegistration, StudioMode};

use crate::{
    exec::{
        ExecJob as StoredExecJob, ExecJobState as StoredExecJobState, ExecLog as StoredExecLog,
        ExecLogLevel as StoredExecLogLevel, ExecValue as StoredExecValue,
    },
    session_id::SessionId,
    snapshot::{
        AppliedPatchSet, InstanceMetadata as RojoInstanceMetadata, InstanceWithMeta, RojoTree,
    },
};

/// Server version to report over the API, not exposed outside this crate.
pub(crate) const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Current protocol version, which is required to match.
pub const PROTOCOL_VERSION: u64 = 5;

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

/// Packet type enum for different websocket message types
#[derive(Debug, Serialize, Deserialize, Display, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
#[strum(serialize_all = "camelCase")]
pub enum SocketPacketType {
    Messages,
    // TODO: Can we cleanly use the socket for all communication?
    // Serialize,
    // RefPatch,
}

/// Body content for messages packet type
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessagesPacket<'a> {
    pub message_cursor: u32,
    pub messages: Vec<SubscribeMessage<'a>>,
}

/// Body content for different packet types
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SocketPacketBody<'a> {
    Messages(MessagesPacket<'a>),
    // TODO: Can we cleanly use the socket for all communication?
    // Serialize(SerializePacket),
    // RefPatch(RefPatchPacket<'a>),
}

/// Message content from /api/socket
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SocketPacket<'a> {
    pub session_id: SessionId,
    pub packet_type: SocketPacketType,
    pub body: SocketPacketBody<'a>,
}

/// Response body from /api/open/{id}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenResponse {
    pub session_id: SessionId,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializeRequest {
    pub session_id: SessionId,
    pub ids: Vec<Ref>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializeResponse {
    pub session_id: SessionId,
    #[serde(with = "serde_bytes")]
    pub model_contents: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefPatchRequest {
    pub session_id: SessionId,
    pub ids: HashSet<Ref>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefPatchResponse<'a> {
    pub session_id: SessionId,
    pub patch: SubscribeMessage<'a>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExecJobSubmissionRequest {
    pub script_name: String,
    pub source: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ExecJobState {
    Pending,
    Claimed,
    Succeeded,
    Failed,
    TimedOut,
}

impl From<StoredExecJobState> for ExecJobState {
    fn from(value: StoredExecJobState) -> Self {
        match value {
            StoredExecJobState::Pending => Self::Pending,
            StoredExecJobState::Claimed => Self::Claimed,
            StoredExecJobState::Succeeded => Self::Succeeded,
            StoredExecJobState::Failed => Self::Failed,
            StoredExecJobState::TimedOut => Self::TimedOut,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ExecValue {
    Nil,
    String { value: String },
    Number { value: f64 },
    Boolean { value: bool },
    Array { value: Vec<ExecValue> },
    Table { value: Vec<ExecTableEntry> },
}

impl From<StoredExecValue> for ExecValue {
    fn from(value: StoredExecValue) -> Self {
        match value {
            StoredExecValue::Nil => Self::Nil,
            StoredExecValue::String(value) => Self::String { value },
            StoredExecValue::Number(value) => Self::Number { value },
            StoredExecValue::Boolean(value) => Self::Boolean { value },
            StoredExecValue::Array(value) => Self::Array {
                value: value.into_iter().map(Into::into).collect(),
            },
            StoredExecValue::Table(value) => Self::Table {
                value: value
                    .into_iter()
                    .map(|(key, value)| ExecTableEntry {
                        key,
                        value: value.into(),
                    })
                    .collect(),
            },
        }
    }
}

impl From<ExecValue> for StoredExecValue {
    fn from(value: ExecValue) -> Self {
        match value {
            ExecValue::Nil => Self::Nil,
            ExecValue::String { value } => Self::String(value),
            ExecValue::Number { value } => Self::Number(value),
            ExecValue::Boolean { value } => Self::Boolean(value),
            ExecValue::Array { value } => Self::Array(value.into_iter().map(Into::into).collect()),
            ExecValue::Table { value } => Self::Table(
                value
                    .into_iter()
                    .map(|entry| (entry.key, entry.value.into()))
                    .collect(),
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExecTableEntry {
    pub key: String,
    pub value: ExecValue,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ExecLogLevel {
    Print,
    Warn,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExecLog {
    pub level: ExecLogLevel,
    pub message: String,
}

impl From<StoredExecLog> for ExecLog {
    fn from(value: StoredExecLog) -> Self {
        Self {
            level: match value.level {
                StoredExecLogLevel::Print => ExecLogLevel::Print,
                StoredExecLogLevel::Warn => ExecLogLevel::Warn,
            },
            message: value.message,
        }
    }
}

impl From<ExecLog> for StoredExecLog {
    fn from(value: ExecLog) -> Self {
        Self {
            level: match value.level {
                ExecLogLevel::Print => StoredExecLogLevel::Print,
                ExecLogLevel::Warn => StoredExecLogLevel::Warn,
            },
            message: value.message,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExecJobResponse {
    pub job_id: String,
    pub script_name: String,
    pub state: ExecJobState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<ExecValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logs: Option<Vec<ExecLog>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub traceback: Option<String>,
}

impl From<StoredExecJob> for ExecJobResponse {
    fn from(value: StoredExecJob) -> Self {
        Self {
            job_id: value.id.to_string(),
            script_name: value.script_name,
            state: value.state.into(),
            result: value.result.map(Into::into),
            logs: value
                .logs
                .map(|logs| logs.into_iter().map(Into::into).collect()),
            error: value.runtime_error,
            traceback: value.traceback,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExecJobClaimResponse {
    pub job_id: String,
    pub script_name: String,
    pub source: String,
    pub state: ExecJobState,
}

impl From<StoredExecJob> for ExecJobClaimResponse {
    fn from(value: StoredExecJob) -> Self {
        Self {
            job_id: value.id.to_string(),
            script_name: value.script_name,
            source: value.source,
            state: value.state.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub enum ExecJobCompletionRequest {
    Success {
        #[serde(default)]
        result: Option<ExecValue>,
        #[serde(default)]
        logs: Vec<ExecLog>,
    },
    CompileFailure {
        error: String,
        #[serde(default)]
        traceback: Option<String>,
        #[serde(default)]
        logs: Vec<ExecLog>,
    },
    RuntimeFailure {
        error: String,
        #[serde(default)]
        traceback: Option<String>,
        #[serde(default)]
        logs: Vec<ExecLog>,
    },
    Rejected {
        error: String,
        #[serde(default)]
        traceback: Option<String>,
        #[serde(default)]
        logs: Vec<ExecLog>,
    },
    Timeout {
        error: String,
        #[serde(default)]
        traceback: Option<String>,
        #[serde(default)]
        logs: Vec<ExecLog>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExecJobCompletionEnvelope {
    #[serde(flatten)]
    pub completion: ExecJobCompletionRequest,
    #[serde(default)]
    pub plugin_session_id: Option<String>,
    #[serde(default)]
    pub studio_mode: Option<StudioMode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AutomationHeartbeatRequest {
    pub plugin_session_id: String,
    pub server_session_id: SessionId,
    pub studio_mode: StudioMode,
    pub exec_handler_available: bool,
    pub automation_handler_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AutomationHeartbeatResponse {
    pub registration: AutomationRegistration,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_plugin_session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AutomationPluginStatusResponse {
    pub connected: bool,
    pub plugin_session_id: String,
    pub studio_mode: StudioMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_version: Option<String>,
    pub automation_handler_version: u32,
    pub last_seen_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AutomationQueueStatusResponse {
    pub exec_pending: usize,
    pub exec_claimed: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exec_claimed_by_plugin_session_id: Option<String>,
    pub automation_pending: usize,
    pub automation_claimed: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automation_claimed_by_plugin_session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AutomationStatusResponse {
    pub server_session_id: SessionId,
    pub server_version: String,
    pub protocol_version: u64,
    pub automation_handler_version: u32,
    pub automation_available: bool,
    pub exec_available: bool,
    pub typed_automation_available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin: Option<AutomationPluginStatusResponse>,
    pub duplicate_session_detected: bool,
    pub queues: AutomationQueueStatusResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AutomationJobResponse {
    pub job_id: String,
    pub state: AutomationJobState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claimed_by_plugin_session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<AutomationResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl From<crate::automation::AutomationJob> for AutomationJobResponse {
    fn from(job: crate::automation::AutomationJob) -> Self {
        Self {
            job_id: job.id.to_string(),
            state: job.state,
            claimed_by_plugin_session_id: job.claimed_by.map(|id| id.to_string()),
            result: job.result,
            error: job.error,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AutomationJobClaimResponse {
    pub job_id: String,
    pub state: AutomationJobState,
    pub request: AutomationRequest,
    pub execution_timeout_ms: u64,
}

impl From<crate::automation::AutomationJob> for AutomationJobClaimResponse {
    fn from(job: crate::automation::AutomationJob) -> Self {
        Self {
            job_id: job.id.to_string(),
            state: job.state,
            request: job.request,
            execution_timeout_ms: job
                .execution_timeout
                .as_millis()
                .try_into()
                .unwrap_or(u64::MAX),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub enum AutomationJobCompletion {
    Success { result: Box<AutomationResult> },
    Failure { error: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AutomationJobCompletionRequest {
    #[serde(flatten)]
    pub completion: AutomationJobCompletion,
    pub plugin_session_id: String,
    pub studio_mode: StudioMode,
}

/// General response type returned from all Rojo routes
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    kind: ErrorResponseKind,
    details: String,
}

impl ErrorResponse {
    pub(crate) fn kind(&self) -> &ErrorResponseKind {
        &self.kind
    }

    pub(crate) fn details(&self) -> &str {
        &self.details
    }

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

    pub fn forbidden<S: Into<String>>(details: S) -> Self {
        Self {
            kind: ErrorResponseKind::Forbidden,
            details: details.into(),
        }
    }

    pub fn conflict<S: Into<String>>(details: S) -> Self {
        Self {
            kind: ErrorResponseKind::Conflict,
            details: details.into(),
        }
    }

    pub fn payload_too_large<S: Into<String>>(details: S) -> Self {
        Self {
            kind: ErrorResponseKind::PayloadTooLarge,
            details: details.into(),
        }
    }

    pub fn too_many_requests<S: Into<String>>(details: S) -> Self {
        Self {
            kind: ErrorResponseKind::TooManyRequests,
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
    Forbidden,
    Conflict,
    PayloadTooLarge,
    TooManyRequests,
    InternalError,
}
