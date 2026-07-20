use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    sync::Mutex,
    time::{Duration, Instant},
};

use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::Deserializer;
use serde::Serializer;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::automation_status::PluginSessionId;

pub const MAX_PENDING_AUTOMATION_JOBS: usize = 16;
pub const MAX_RETAINED_AUTOMATION_JOBS: usize = 64;
pub const AUTOMATION_CLAIM_TIMEOUT: Duration = Duration::from_secs(30);
pub const INSPECT_EXECUTION_TIMEOUT: Duration = Duration::from_secs(15);
pub const AUTOMATION_TERMINAL_RETENTION: Duration = Duration::from_secs(5 * 60);
pub const MAX_AUTOMATION_REQUEST_BODY_BYTES: usize = 256 * 1024;
pub const MAX_AUTOMATION_RESULT_BODY_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_INSPECT_DEPTH: u8 = 8;
pub const MAX_INSPECT_CHILDREN: u32 = 1_000;
pub const MAX_INSPECT_INSTANCES: u32 = 10_000;
pub const MAX_AUTOMATION_STRING_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AutomationJobState {
    Pending,
    Claimed,
    Succeeded,
    Failed,
    TimedOut,
}

impl AutomationJobState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::TimedOut)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AutomationRequest {
    Inspect(InspectRequest),
}

impl AutomationRequest {
    fn execution_timeout(&self) -> Duration {
        match self {
            Self::Inspect(_) => INSPECT_EXECUTION_TIMEOUT,
        }
    }

    fn accepts_result(&self, result: &AutomationResult) -> bool {
        matches!(
            (self, result),
            (Self::Inspect(_), AutomationResult::Inspect(_))
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum InspectTarget {
    Path { segments: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectRequest {
    pub target: InspectTarget,
    pub depth: u8,
    pub max_children: u32,
    pub max_instances: u32,
    pub include_properties: bool,
    pub include_attributes: bool,
    pub include_tags: bool,
}

impl InspectRequest {
    pub fn validate(&self) -> Result<(), AutomationJobStoreError> {
        let InspectTarget::Path { segments } = &self.target;
        if segments.is_empty() || segments.iter().any(String::is_empty) {
            return Err(AutomationJobStoreError::InvalidRequest(
                "inspect target must contain at least one non-empty path segment".to_owned(),
            ));
        }
        if self.depth > MAX_INSPECT_DEPTH {
            return Err(AutomationJobStoreError::InvalidRequest(format!(
                "inspect depth {} exceeds the maximum of {MAX_INSPECT_DEPTH}",
                self.depth
            )));
        }
        if self.max_children == 0 || self.max_children > MAX_INSPECT_CHILDREN {
            return Err(AutomationJobStoreError::InvalidRequest(format!(
                "inspect maxChildren must be between 1 and {MAX_INSPECT_CHILDREN}"
            )));
        }
        if self.max_instances == 0 || self.max_instances > MAX_INSPECT_INSTANCES {
            return Err(AutomationJobStoreError::InvalidRequest(format!(
                "inspect maxInstances must be between 1 and {MAX_INSPECT_INSTANCES}"
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AutomationResult {
    Inspect(InspectResult),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectResult {
    pub root: InspectNode,
    pub visited_instances: u32,
    pub truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncation_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectNode {
    pub reference: InstanceReference,
    pub name: String,
    pub class_name: String,
    pub path: String,
    #[serde(
        default,
        with = "string_map_or_empty_array",
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    pub properties: BTreeMap<String, AutomationValue>,
    #[serde(
        default,
        with = "string_map_or_empty_array",
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    pub attributes: BTreeMap<String, AutomationValue>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<InspectNode>,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceReference {
    pub session_id: String,
    pub id: String,
    pub path: String,
    pub name: String,
    pub class_name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AutomationValue {
    Nil,
    Boolean {
        value: bool,
    },
    Number {
        value: f64,
    },
    String {
        value: String,
    },
    Array {
        value: Vec<AutomationValue>,
    },
    Map {
        value: Vec<AutomationMapEntry>,
    },
    Vector2 {
        x: f64,
        y: f64,
    },
    Vector3 {
        x: f64,
        y: f64,
        z: f64,
    },
    CFrame {
        components: Vec<f64>,
    },
    Color3 {
        r: f64,
        g: f64,
        b: f64,
    },
    UDim {
        scale: f64,
        offset: i32,
    },
    UDim2 {
        x: AutomationUdim,
        y: AutomationUdim,
    },
    Rect {
        min: AutomationVector2,
        max: AutomationVector2,
    },
    EnumItem {
        enum_type: String,
        name: String,
    },
    BrickColor {
        number: u16,
        name: String,
    },
    NumberRange {
        min: f64,
        max: f64,
    },
    InstanceReference {
        value: InstanceReference,
    },
    Diagnostic {
        error: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationMapEntry {
    pub key: String,
    pub value: AutomationValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationUdim {
    pub scale: f64,
    pub offset: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationVector2 {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AutomationJob {
    pub id: Uuid,
    pub request: AutomationRequest,
    pub state: AutomationJobState,
    pub claimed_by: Option<PluginSessionId>,
    pub execution_timeout: Duration,
    pub result: Option<AutomationResult>,
    pub error: Option<String>,
    claim_deadline: Instant,
    execution_deadline: Option<Instant>,
    terminal_at: Option<Instant>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AutomationQueueCounts {
    pub pending: usize,
    pub claimed: usize,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AutomationJobStoreError {
    #[error("invalid automation request: {0}")]
    InvalidRequest(String),
    #[error("automation pending queue is full (limit: {limit})")]
    PendingQueueFull { limit: usize },
    #[error("automation job {id} does not exist")]
    UnknownJob { id: Uuid },
    #[error("automation job {id} cannot be completed from state {state:?}")]
    JobNotClaimed { id: Uuid, state: AutomationJobState },
    #[error("automation job {id} was already completed with state {state:?}")]
    DuplicateCompletion { id: Uuid, state: AutomationJobState },
    #[error("automation job {id} was claimed by a different plugin session")]
    WrongClaimant { id: Uuid },
    #[error("automation result kind does not match job {id}")]
    ResultKindMismatch { id: Uuid },
}

#[derive(Debug)]
pub struct AutomationJobStore {
    inner: Mutex<AutomationJobStoreInner>,
    claim_timeout: Duration,
    terminal_retention: Duration,
}

#[derive(Debug, Default)]
struct AutomationJobStoreInner {
    jobs: HashMap<Uuid, AutomationJob>,
    pending: VecDeque<Uuid>,
    claimed: Option<Uuid>,
    terminal: VecDeque<Uuid>,
}

impl AutomationJobStore {
    pub fn new() -> Self {
        Self::with_durations(AUTOMATION_CLAIM_TIMEOUT, AUTOMATION_TERMINAL_RETENTION)
    }

    pub fn submit(
        &self,
        request: AutomationRequest,
    ) -> Result<AutomationJob, AutomationJobStoreError> {
        self.submit_at(request, Instant::now())
    }

    pub fn claim_next(&self, claimant: PluginSessionId) -> Option<AutomationJob> {
        self.claim_next_at(claimant, Instant::now())
    }

    pub fn complete_success(
        &self,
        id: Uuid,
        claimant: PluginSessionId,
        result: AutomationResult,
    ) -> Result<AutomationJob, AutomationJobStoreError> {
        self.complete_at(id, claimant, Some(result), None, Instant::now())
    }

    pub fn complete_failure(
        &self,
        id: Uuid,
        claimant: PluginSessionId,
        error: String,
    ) -> Result<AutomationJob, AutomationJobStoreError> {
        self.complete_at(id, claimant, None, Some(error), Instant::now())
    }

    pub fn get(&self, id: Uuid) -> Result<AutomationJob, AutomationJobStoreError> {
        self.inner
            .lock()
            .unwrap()
            .jobs
            .get(&id)
            .cloned()
            .ok_or(AutomationJobStoreError::UnknownJob { id })
    }

    pub fn queue_counts(&self) -> AutomationQueueCounts {
        let inner = self.inner.lock().unwrap();
        AutomationQueueCounts {
            pending: inner.pending.len(),
            claimed: usize::from(inner.claimed.is_some()),
        }
    }

    pub fn claimed_by(&self) -> Option<PluginSessionId> {
        let inner = self.inner.lock().unwrap();
        inner
            .claimed
            .and_then(|id| inner.jobs.get(&id))
            .and_then(|job| job.claimed_by)
    }

    pub fn cleanup_expired(&self) {
        self.cleanup_expired_at(Instant::now());
    }

    fn with_durations(claim_timeout: Duration, terminal_retention: Duration) -> Self {
        Self {
            inner: Mutex::new(AutomationJobStoreInner::default()),
            claim_timeout,
            terminal_retention,
        }
    }

    fn submit_at(
        &self,
        request: AutomationRequest,
        now: Instant,
    ) -> Result<AutomationJob, AutomationJobStoreError> {
        match &request {
            AutomationRequest::Inspect(request) => request.validate()?,
        }
        let mut inner = self.inner.lock().unwrap();
        if inner.pending.len() + usize::from(inner.claimed.is_some()) >= MAX_PENDING_AUTOMATION_JOBS
        {
            return Err(AutomationJobStoreError::PendingQueueFull {
                limit: MAX_PENDING_AUTOMATION_JOBS,
            });
        }
        let id = Uuid::new_v4();
        let execution_timeout = request.execution_timeout();
        let job = AutomationJob {
            id,
            request,
            state: AutomationJobState::Pending,
            claimed_by: None,
            execution_timeout,
            result: None,
            error: None,
            claim_deadline: now + self.claim_timeout,
            execution_deadline: None,
            terminal_at: None,
        };
        inner.jobs.insert(id, job.clone());
        inner.pending.push_back(id);
        Ok(job)
    }

    fn claim_next_at(&self, claimant: PluginSessionId, now: Instant) -> Option<AutomationJob> {
        let mut inner = self.inner.lock().unwrap();
        if inner.claimed.is_some() {
            return None;
        }
        while let Some(id) = inner.pending.pop_front() {
            let Some(job) = inner.jobs.get_mut(&id) else {
                continue;
            };
            if job.state != AutomationJobState::Pending {
                continue;
            }
            job.state = AutomationJobState::Claimed;
            job.claimed_by = Some(claimant);
            job.execution_deadline = Some(now + job.execution_timeout);
            let claimed = job.clone();
            inner.claimed = Some(id);
            return Some(claimed);
        }
        None
    }

    fn complete_at(
        &self,
        id: Uuid,
        claimant: PluginSessionId,
        result: Option<AutomationResult>,
        error: Option<String>,
        now: Instant,
    ) -> Result<AutomationJob, AutomationJobStoreError> {
        let mut inner = self.inner.lock().unwrap();
        let job = inner
            .jobs
            .get(&id)
            .ok_or(AutomationJobStoreError::UnknownJob { id })?;
        if job.state.is_terminal() {
            return Err(AutomationJobStoreError::DuplicateCompletion {
                id,
                state: job.state,
            });
        }
        if job.state != AutomationJobState::Claimed {
            return Err(AutomationJobStoreError::JobNotClaimed {
                id,
                state: job.state,
            });
        }
        if job.claimed_by != Some(claimant) {
            return Err(AutomationJobStoreError::WrongClaimant { id });
        }
        if let Some(result) = &result {
            if !job.request.accepts_result(result) {
                return Err(AutomationJobStoreError::ResultKindMismatch { id });
            }
        }

        let job = inner.jobs.get_mut(&id).unwrap();
        job.state = if result.is_some() {
            AutomationJobState::Succeeded
        } else {
            AutomationJobState::Failed
        };
        job.result = result;
        job.error = error;
        job.terminal_at = Some(now);
        let completed = job.clone();
        inner.claimed = None;
        inner.terminal.push_back(id);
        enforce_terminal_limit(&mut inner);
        Ok(completed)
    }

    fn cleanup_expired_at(&self, now: Instant) {
        let mut inner = self.inner.lock().unwrap();
        let pending_count = inner.pending.len();
        for _ in 0..pending_count {
            let id = inner.pending.pop_front().unwrap();
            let expired = inner.jobs.get(&id).is_some_and(|job| {
                job.state == AutomationJobState::Pending && now >= job.claim_deadline
            });
            if expired {
                time_out(&mut inner, id, now);
            } else if inner.jobs.contains_key(&id) {
                inner.pending.push_back(id);
            }
        }
        if let Some(id) = inner.claimed {
            let expired = inner.jobs.get(&id).is_some_and(|job| {
                job.execution_deadline
                    .is_some_and(|deadline| now >= deadline)
            });
            if expired {
                time_out(&mut inner, id, now);
                inner.claimed = None;
            }
        }
        while let Some(id) = inner.terminal.front().copied() {
            let remove = inner.jobs.get(&id).is_none_or(|job| {
                job.terminal_at.is_some_and(|terminal| {
                    now.saturating_duration_since(terminal) >= self.terminal_retention
                })
            });
            if !remove {
                break;
            }
            inner.terminal.pop_front();
            inner.jobs.remove(&id);
        }
        enforce_terminal_limit(&mut inner);
    }
}

impl Default for AutomationJobStore {
    fn default() -> Self {
        Self::new()
    }
}

fn time_out(inner: &mut AutomationJobStoreInner, id: Uuid, now: Instant) {
    if let Some(job) = inner.jobs.get_mut(&id) {
        job.state = AutomationJobState::TimedOut;
        job.error = Some("automation job timed out".to_owned());
        job.terminal_at = Some(now);
        inner.terminal.push_back(id);
        enforce_terminal_limit(inner);
    }
}

fn enforce_terminal_limit(inner: &mut AutomationJobStoreInner) {
    while inner.terminal.len() > MAX_RETAINED_AUTOMATION_JOBS {
        if let Some(id) = inner.terminal.pop_front() {
            inner.jobs.remove(&id);
        }
    }
}

mod string_map_or_empty_array {
    use super::*;
    use std::fmt;

    pub fn serialize<S>(
        value: &BTreeMap<String, AutomationValue>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        BTreeMap::serialize(value, serializer)
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<BTreeMap<String, AutomationValue>, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(MapVisitor)
    }

    struct MapVisitor;

    impl<'de> Visitor<'de> for MapVisitor {
        type Value = BTreeMap<String, AutomationValue>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a map, or an empty array")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut output = BTreeMap::new();
            while let Some((key, value)) = map.next_entry::<String, AutomationValue>()? {
                output.insert(key, value);
            }
            Ok(output)
        }

        fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            if sequence.next_element::<de::IgnoredAny>()?.is_some() {
                return Err(de::Error::invalid_length(1, &"an empty array"));
            }
            Ok(BTreeMap::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inspect(name: &str) -> AutomationRequest {
        AutomationRequest::Inspect(InspectRequest {
            target: InspectTarget::Path {
                segments: vec![name.to_owned()],
            },
            depth: 1,
            max_children: 100,
            max_instances: 2_000,
            include_properties: false,
            include_attributes: true,
            include_tags: true,
        })
    }

    fn result(name: &str) -> AutomationResult {
        AutomationResult::Inspect(InspectResult {
            root: InspectNode {
                reference: InstanceReference {
                    session_id: "session".to_owned(),
                    id: "pinst-00000001".to_owned(),
                    path: name.to_owned(),
                    name: name.to_owned(),
                    class_name: "Workspace".to_owned(),
                },
                name: name.to_owned(),
                class_name: "Workspace".to_owned(),
                path: name.to_owned(),
                properties: BTreeMap::new(),
                attributes: BTreeMap::new(),
                tags: Vec::new(),
                children: Vec::new(),
                truncated: false,
            },
            visited_instances: 1,
            truncated: false,
            truncation_reason: None,
        })
    }

    #[test]
    fn submit_claim_fifo_and_authoritative_completion() {
        let store = AutomationJobStore::new();
        let first = store.submit(inspect("Workspace")).unwrap();
        let second = store.submit(inspect("Lighting")).unwrap();
        let claimant: PluginSessionId = Uuid::new_v4().to_string().parse().unwrap();
        let other: PluginSessionId = Uuid::new_v4().to_string().parse().unwrap();
        assert_eq!(store.claim_next(claimant).unwrap().id, first.id);
        assert_eq!(store.claim_next(claimant), None);
        assert_eq!(
            store.complete_success(first.id, other, result("Workspace")),
            Err(AutomationJobStoreError::WrongClaimant { id: first.id })
        );
        let completed = store
            .complete_success(first.id, claimant, result("Workspace"))
            .unwrap();
        assert_eq!(completed.state, AutomationJobState::Succeeded);
        assert_eq!(store.claim_next(claimant).unwrap().id, second.id);
    }

    #[test]
    fn rejects_completion_before_claim_duplicate_and_unknown() {
        let store = AutomationJobStore::new();
        let job = store.submit(inspect("Workspace")).unwrap();
        let claimant: PluginSessionId = Uuid::new_v4().to_string().parse().unwrap();
        assert!(matches!(
            store.complete_failure(job.id, claimant, "no".to_owned()),
            Err(AutomationJobStoreError::JobNotClaimed { .. })
        ));
        store.claim_next(claimant).unwrap();
        store
            .complete_failure(job.id, claimant, "failed".to_owned())
            .unwrap();
        assert!(matches!(
            store.complete_failure(job.id, claimant, "again".to_owned()),
            Err(AutomationJobStoreError::DuplicateCompletion { .. })
        ));
        assert!(matches!(
            store.get(Uuid::new_v4()),
            Err(AutomationJobStoreError::UnknownJob { .. })
        ));
    }

    #[test]
    fn times_out_and_cleans_up_jobs() {
        let store =
            AutomationJobStore::with_durations(Duration::from_secs(1), Duration::from_secs(1));
        let now = Instant::now();
        let pending = store.submit_at(inspect("Workspace"), now).unwrap();
        store.cleanup_expired_at(now + Duration::from_secs(1));
        assert_eq!(
            store.get(pending.id).unwrap().state,
            AutomationJobState::TimedOut
        );
        store.cleanup_expired_at(now + Duration::from_secs(2));
        assert!(matches!(
            store.get(pending.id),
            Err(AutomationJobStoreError::UnknownJob { .. })
        ));
    }

    #[test]
    fn times_out_claimed_jobs_and_rejects_late_completion() {
        let store =
            AutomationJobStore::with_durations(Duration::from_secs(30), Duration::from_secs(30));
        let now = Instant::now();
        let job = store.submit_at(inspect("Workspace"), now).unwrap();
        let claimant = PluginSessionId::new();
        store.claim_next_at(claimant, now).unwrap();
        store.cleanup_expired_at(now + INSPECT_EXECUTION_TIMEOUT);
        assert_eq!(
            store.get(job.id).unwrap().state,
            AutomationJobState::TimedOut
        );
        assert!(matches!(
            store.complete_success(job.id, claimant, result("Workspace")),
            Err(AutomationJobStoreError::DuplicateCompletion {
                state: AutomationJobState::TimedOut,
                ..
            })
        ));
    }

    #[test]
    fn enforces_queue_and_request_limits() {
        let store = AutomationJobStore::new();
        for _ in 0..MAX_PENDING_AUTOMATION_JOBS {
            store.submit(inspect("Workspace")).unwrap();
        }
        assert!(matches!(
            store.submit(inspect("Workspace")),
            Err(AutomationJobStoreError::PendingQueueFull { .. })
        ));
        let mut invalid = inspect("Workspace");
        let AutomationRequest::Inspect(request) = &mut invalid;
        request.depth = MAX_INSPECT_DEPTH + 1;
        assert!(matches!(
            AutomationJobStore::new().submit(invalid),
            Err(AutomationJobStoreError::InvalidRequest(_))
        ));
    }
}
