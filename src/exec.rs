use std::{
    collections::{HashMap, VecDeque},
    sync::Mutex,
    time::{Duration, Instant, SystemTime},
};

use thiserror::Error;
use uuid::Uuid;

pub const MAX_SOURCE_SIZE_BYTES: usize = 1024 * 1024;
pub const MAX_PENDING_JOBS: usize = 16;
pub const MAX_RETAINED_TERMINAL_JOBS: usize = 64;

pub const CLAIM_TIMEOUT: Duration = Duration::from_secs(30);
pub const EXECUTION_TIMEOUT: Duration = Duration::from_secs(30);
pub const TERMINAL_RETENTION: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecJobState {
    Pending,
    Claimed,
    Succeeded,
    Failed,
    TimedOut,
}

impl ExecJobState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::TimedOut)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExecValue {
    Nil,
    String(String),
    Number(f64),
    Boolean(bool),
    Array(Vec<ExecValue>),
    Table(Vec<(String, ExecValue)>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecLogLevel {
    Print,
    Warn,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecLog {
    pub level: ExecLogLevel,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExecJob {
    pub id: Uuid,
    pub script_name: String,
    pub source: String,
    pub state: ExecJobState,
    pub created_at: SystemTime,
    pub claim_deadline: Instant,
    pub execution_deadline: Option<Instant>,
    pub result: Option<ExecValue>,
    pub logs: Option<Vec<ExecLog>>,
    pub runtime_error: Option<String>,
    pub traceback: Option<String>,

    terminal_at: Option<Instant>,
}

impl ExecJob {
    pub fn is_terminal(&self) -> bool {
        self.state.is_terminal()
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ExecJobStoreError {
    #[error("exec source is {size} bytes, exceeding the {limit}-byte limit")]
    SourceTooLarge { size: usize, limit: usize },

    #[error("exec pending queue is full (limit: {limit})")]
    PendingQueueFull { limit: usize },

    #[error("exec job {id} does not exist")]
    UnknownJob { id: Uuid },

    #[error("exec job {id} cannot be completed from state {state:?}")]
    JobNotClaimed { id: Uuid, state: ExecJobState },

    #[error("exec job {id} was already completed with state {state:?}")]
    DuplicateCompletion { id: Uuid, state: ExecJobState },
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ExecCleanupStats {
    pub timed_out_pending: usize,
    pub timed_out_claimed: usize,
    pub removed_terminal: usize,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ExecQueueCounts {
    pub pending: usize,
    pub claimed: usize,
}

#[derive(Debug)]
pub struct ExecJobStore {
    inner: Mutex<ExecJobStoreInner>,
    claim_timeout: Duration,
    execution_timeout: Duration,
    terminal_retention: Duration,
}

#[derive(Debug, Default)]
struct ExecJobStoreInner {
    jobs: HashMap<Uuid, ExecJob>,
    pending_jobs: VecDeque<Uuid>,
    claimed_job: Option<Uuid>,
    terminal_jobs: VecDeque<Uuid>,
}

impl ExecJobStore {
    pub fn new() -> Self {
        Self::with_durations(CLAIM_TIMEOUT, EXECUTION_TIMEOUT, TERMINAL_RETENTION)
    }

    pub fn submit(
        &self,
        script_name: impl Into<String>,
        source: impl Into<String>,
    ) -> Result<ExecJob, ExecJobStoreError> {
        self.submit_at(
            script_name.into(),
            source.into(),
            SystemTime::now(),
            Instant::now(),
        )
    }

    pub fn claim_next(&self) -> Option<ExecJob> {
        self.claim_next_at(Instant::now())
    }

    pub fn complete_success(
        &self,
        id: Uuid,
        result: Option<ExecValue>,
        logs: Option<Vec<ExecLog>>,
    ) -> Result<ExecJob, ExecJobStoreError> {
        self.complete_success_at(id, result, logs, Instant::now())
    }

    pub fn complete_failure(
        &self,
        id: Uuid,
        runtime_error: Option<String>,
        traceback: Option<String>,
        logs: Option<Vec<ExecLog>>,
    ) -> Result<ExecJob, ExecJobStoreError> {
        self.complete_failure_at(id, runtime_error, traceback, logs, Instant::now())
    }

    pub fn complete_timeout(
        &self,
        id: Uuid,
        runtime_error: Option<String>,
        traceback: Option<String>,
        logs: Option<Vec<ExecLog>>,
    ) -> Result<ExecJob, ExecJobStoreError> {
        self.complete_timeout_at(id, runtime_error, traceback, logs, Instant::now())
    }

    pub fn get(&self, id: Uuid) -> Result<ExecJob, ExecJobStoreError> {
        let inner = self.inner.lock().unwrap();

        inner
            .jobs
            .get(&id)
            .cloned()
            .ok_or(ExecJobStoreError::UnknownJob { id })
    }

    pub fn cleanup_expired(&self) -> ExecCleanupStats {
        self.cleanup_expired_at(Instant::now())
    }

    pub fn queue_counts(&self) -> ExecQueueCounts {
        let inner = self.inner.lock().unwrap();
        ExecQueueCounts {
            pending: inner.pending_jobs.len(),
            claimed: usize::from(inner.claimed_job.is_some()),
        }
    }

    fn with_durations(
        claim_timeout: Duration,
        execution_timeout: Duration,
        terminal_retention: Duration,
    ) -> Self {
        Self {
            inner: Mutex::new(ExecJobStoreInner::default()),
            claim_timeout,
            execution_timeout,
            terminal_retention,
        }
    }

    fn submit_at(
        &self,
        script_name: String,
        source: String,
        created_at: SystemTime,
        now: Instant,
    ) -> Result<ExecJob, ExecJobStoreError> {
        let source_size = source.len();
        if source_size > MAX_SOURCE_SIZE_BYTES {
            return Err(ExecJobStoreError::SourceTooLarge {
                size: source_size,
                limit: MAX_SOURCE_SIZE_BYTES,
            });
        }

        let mut inner = self.inner.lock().unwrap();
        if inner.pending_jobs.len() >= MAX_PENDING_JOBS {
            return Err(ExecJobStoreError::PendingQueueFull {
                limit: MAX_PENDING_JOBS,
            });
        }

        let id = loop {
            let candidate = Uuid::new_v4();
            if !inner.jobs.contains_key(&candidate) {
                break candidate;
            }
        };

        let job = ExecJob {
            id,
            script_name,
            source,
            state: ExecJobState::Pending,
            created_at,
            claim_deadline: now + self.claim_timeout,
            execution_deadline: None,
            result: None,
            logs: None,
            runtime_error: None,
            traceback: None,
            terminal_at: None,
        };

        inner.jobs.insert(id, job.clone());
        inner.pending_jobs.push_back(id);

        Ok(job)
    }

    fn claim_next_at(&self, now: Instant) -> Option<ExecJob> {
        let mut inner = self.inner.lock().unwrap();
        if inner.claimed_job.is_some() {
            return None;
        }

        while let Some(id) = inner.pending_jobs.pop_front() {
            let claimed_job = {
                let Some(job) = inner.jobs.get_mut(&id) else {
                    continue;
                };

                if job.state != ExecJobState::Pending {
                    continue;
                }

                job.state = ExecJobState::Claimed;
                job.execution_deadline = Some(now + self.execution_timeout);
                job.clone()
            };

            inner.claimed_job = Some(id);
            return Some(claimed_job);
        }

        None
    }

    fn complete_success_at(
        &self,
        id: Uuid,
        result: Option<ExecValue>,
        logs: Option<Vec<ExecLog>>,
        now: Instant,
    ) -> Result<ExecJob, ExecJobStoreError> {
        let mut inner = self.inner.lock().unwrap();
        ensure_job_can_complete(&inner, id)?;

        {
            let job = inner.jobs.get_mut(&id).unwrap();
            job.state = ExecJobState::Succeeded;
            job.source.clear();
            job.result = result;
            job.logs = logs;
            job.runtime_error = None;
            job.traceback = None;
            job.terminal_at = Some(now);
        }

        inner.claimed_job = None;
        track_terminal_job(&mut inner, id);

        Ok(inner.jobs.get(&id).unwrap().clone())
    }

    fn complete_failure_at(
        &self,
        id: Uuid,
        runtime_error: Option<String>,
        traceback: Option<String>,
        logs: Option<Vec<ExecLog>>,
        now: Instant,
    ) -> Result<ExecJob, ExecJobStoreError> {
        let mut inner = self.inner.lock().unwrap();
        ensure_job_can_complete(&inner, id)?;

        {
            let job = inner.jobs.get_mut(&id).unwrap();
            job.state = ExecJobState::Failed;
            job.source.clear();
            job.result = None;
            job.logs = logs;
            job.runtime_error = runtime_error;
            job.traceback = traceback;
            job.terminal_at = Some(now);
        }

        inner.claimed_job = None;
        track_terminal_job(&mut inner, id);

        Ok(inner.jobs.get(&id).unwrap().clone())
    }

    fn complete_timeout_at(
        &self,
        id: Uuid,
        runtime_error: Option<String>,
        traceback: Option<String>,
        logs: Option<Vec<ExecLog>>,
        now: Instant,
    ) -> Result<ExecJob, ExecJobStoreError> {
        let mut inner = self.inner.lock().unwrap();
        ensure_job_can_complete(&inner, id)?;

        {
            let job = inner.jobs.get_mut(&id).unwrap();
            job.state = ExecJobState::TimedOut;
            job.source.clear();
            job.result = None;
            job.logs = logs;
            job.runtime_error = runtime_error;
            job.traceback = traceback;
            job.terminal_at = Some(now);
        }

        inner.claimed_job = None;
        track_terminal_job(&mut inner, id);

        Ok(inner.jobs.get(&id).unwrap().clone())
    }

    fn cleanup_expired_at(&self, now: Instant) -> ExecCleanupStats {
        let mut inner = self.inner.lock().unwrap();
        let mut stats = ExecCleanupStats::default();

        let pending_count = inner.pending_jobs.len();
        for _ in 0..pending_count {
            let id = inner.pending_jobs.pop_front().unwrap();
            let is_expired = inner
                .jobs
                .get(&id)
                .is_some_and(|job| job.state == ExecJobState::Pending && now >= job.claim_deadline);

            if is_expired {
                stats.removed_terminal += mark_job_timed_out(&mut inner, id, now);
                stats.timed_out_pending += 1;
            } else if inner.jobs.contains_key(&id) {
                inner.pending_jobs.push_back(id);
            }
        }

        if let Some(id) = inner.claimed_job {
            let is_expired = inner.jobs.get(&id).is_some_and(|job| {
                job.state == ExecJobState::Claimed
                    && job
                        .execution_deadline
                        .is_some_and(|deadline| now >= deadline)
            });

            if is_expired {
                stats.removed_terminal += mark_job_timed_out(&mut inner, id, now);
                inner.claimed_job = None;
                stats.timed_out_claimed += 1;
            }
        }

        while let Some(id) = inner.terminal_jobs.front().copied() {
            let should_remove = match inner.jobs.get(&id) {
                Some(job) => job.terminal_at.is_some_and(|terminal_at| {
                    now.saturating_duration_since(terminal_at) >= self.terminal_retention
                }),
                None => true,
            };

            if !should_remove {
                break;
            }

            inner.terminal_jobs.pop_front();
            if inner.jobs.remove(&id).is_some() {
                stats.removed_terminal += 1;
            }
        }

        stats.removed_terminal += enforce_terminal_limit(&mut inner);
        stats
    }
}

impl Default for ExecJobStore {
    fn default() -> Self {
        Self::new()
    }
}

fn ensure_job_can_complete(inner: &ExecJobStoreInner, id: Uuid) -> Result<(), ExecJobStoreError> {
    let job = inner
        .jobs
        .get(&id)
        .ok_or(ExecJobStoreError::UnknownJob { id })?;

    match job.state {
        ExecJobState::Claimed => Ok(()),
        state if state.is_terminal() => Err(ExecJobStoreError::DuplicateCompletion { id, state }),
        state => Err(ExecJobStoreError::JobNotClaimed { id, state }),
    }
}

fn mark_job_timed_out(inner: &mut ExecJobStoreInner, id: Uuid, now: Instant) -> usize {
    let job = inner
        .jobs
        .get_mut(&id)
        .expect("queued exec job should exist in the job map");

    job.state = ExecJobState::TimedOut;
    job.source.clear();
    job.result = None;
    job.logs = None;
    job.runtime_error = None;
    job.traceback = None;
    job.terminal_at = Some(now);

    track_terminal_job(inner, id)
}

fn track_terminal_job(inner: &mut ExecJobStoreInner, id: Uuid) -> usize {
    inner.terminal_jobs.push_back(id);
    enforce_terminal_limit(inner)
}

fn enforce_terminal_limit(inner: &mut ExecJobStoreInner) -> usize {
    let mut removed = 0;

    while inner.terminal_jobs.len() > MAX_RETAINED_TERMINAL_JOBS {
        let id = inner.terminal_jobs.pop_front().unwrap();
        if inner.jobs.remove(&id).is_some() {
            removed += 1;
        }
    }

    removed
}

#[cfg(test)]
mod test {
    use std::sync::{Arc, Barrier};

    use super::*;

    const TEST_CLAIM_TIMEOUT: Duration = Duration::from_secs(10);
    const TEST_EXECUTION_TIMEOUT: Duration = Duration::from_secs(20);
    const TEST_TERMINAL_RETENTION: Duration = Duration::from_secs(30);

    fn test_store() -> ExecJobStore {
        ExecJobStore::with_durations(
            TEST_CLAIM_TIMEOUT,
            TEST_EXECUTION_TIMEOUT,
            TEST_TERMINAL_RETENTION,
        )
    }

    fn submit_at(store: &ExecJobStore, name: &str, source: &str, now: Instant) -> ExecJob {
        store
            .submit_at(
                name.to_owned(),
                source.to_owned(),
                SystemTime::UNIX_EPOCH,
                now,
            )
            .unwrap()
    }

    #[test]
    fn submit_adds_a_pending_job() {
        let store = test_store();
        let now = Instant::now();
        let job = submit_at(&store, "hello.lua", "return 'hello'", now);

        assert_ne!(job.id, Uuid::nil());
        assert_eq!(job.script_name, "hello.lua");
        assert_eq!(job.source, "return 'hello'");
        assert_eq!(job.state, ExecJobState::Pending);
        assert_eq!(job.created_at, SystemTime::UNIX_EPOCH);
        assert_eq!(job.claim_deadline, now + TEST_CLAIM_TIMEOUT);
        assert_eq!(job.execution_deadline, None);
        assert_eq!(job.result, None);
        assert_eq!(job.logs, None);
        assert_eq!(job.runtime_error, None);
        assert_eq!(job.traceback, None);
        assert_eq!(store.get(job.id).unwrap(), job);
    }

    #[test]
    fn claim_returns_the_oldest_pending_job() {
        let store = test_store();
        let now = Instant::now();
        let first = submit_at(&store, "first.lua", "return 1", now);
        submit_at(&store, "second.lua", "return 2", now);

        let claimed = store.claim_next_at(now + Duration::from_secs(1)).unwrap();

        assert_eq!(claimed.id, first.id);
        assert_eq!(claimed.state, ExecJobState::Claimed);
        assert_eq!(
            claimed.execution_deadline,
            Some(now + Duration::from_secs(1) + TEST_EXECUTION_TIMEOUT)
        );
        assert_eq!(store.get(first.id).unwrap(), claimed);
    }

    #[test]
    fn claim_prevents_a_second_active_claim() {
        let store = test_store();
        let now = Instant::now();
        let first = submit_at(&store, "first.lua", "return 1", now);
        let second = submit_at(&store, "second.lua", "return 2", now);

        assert_eq!(store.claim_next_at(now).unwrap().id, first.id);
        assert_eq!(store.claim_next_at(now), None);
        assert_eq!(store.get(second.id).unwrap().state, ExecJobState::Pending);

        store
            .complete_success_at(first.id, None, None, now)
            .unwrap();
        assert_eq!(store.claim_next_at(now).unwrap().id, second.id);
    }

    #[test]
    fn concurrent_claims_only_return_one_job() {
        let store = Arc::new(test_store());
        let now = Instant::now();
        let job = submit_at(&store, "atomic.lua", "return nil", now);
        let start = Arc::new(Barrier::new(9));
        let mut workers = Vec::new();

        for _ in 0..8 {
            let store = Arc::clone(&store);
            let start = Arc::clone(&start);
            workers.push(std::thread::spawn(move || {
                start.wait();
                store.claim_next_at(now)
            }));
        }

        start.wait();
        let claims: Vec<_> = workers
            .into_iter()
            .filter_map(|worker| worker.join().unwrap())
            .collect();

        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].id, job.id);
    }

    #[test]
    fn complete_success_stores_result_and_logs() {
        let store = test_store();
        let now = Instant::now();
        let job = submit_at(&store, "success.lua", "return true", now);
        store.claim_next_at(now).unwrap();
        let logs = vec![ExecLog {
            level: ExecLogLevel::Print,
            message: "done".to_owned(),
        }];

        let completed = store
            .complete_success_at(
                job.id,
                Some(ExecValue::Boolean(true)),
                Some(logs.clone()),
                now,
            )
            .unwrap();

        assert_eq!(completed.state, ExecJobState::Succeeded);
        assert!(completed.source.is_empty());
        assert_eq!(completed.result, Some(ExecValue::Boolean(true)));
        assert_eq!(completed.logs, Some(logs));
        assert_eq!(completed.runtime_error, None);
        assert_eq!(completed.traceback, None);
    }

    #[test]
    fn complete_failure_stores_error_traceback_and_logs() {
        let store = test_store();
        let now = Instant::now();
        let job = submit_at(&store, "failure.lua", "error('oh no')", now);
        store.claim_next_at(now).unwrap();
        let logs = vec![ExecLog {
            level: ExecLogLevel::Warn,
            message: "before failure".to_owned(),
        }];

        let completed = store
            .complete_failure_at(
                job.id,
                Some("oh no".to_owned()),
                Some("traceback".to_owned()),
                Some(logs.clone()),
                now,
            )
            .unwrap();

        assert_eq!(completed.state, ExecJobState::Failed);
        assert!(completed.source.is_empty());
        assert_eq!(completed.result, None);
        assert_eq!(completed.logs, Some(logs));
        assert_eq!(completed.runtime_error.as_deref(), Some("oh no"));
        assert_eq!(completed.traceback.as_deref(), Some("traceback"));
    }

    #[test]
    fn complete_timeout_stores_timeout_details() {
        let store = test_store();
        let now = Instant::now();
        let job = submit_at(&store, "timeout.lua", "task.wait(60)", now);
        store.claim_next_at(now).unwrap();
        let logs = vec![ExecLog {
            level: ExecLogLevel::Warn,
            message: "still running".to_owned(),
        }];

        let completed = store
            .complete_timeout_at(
                job.id,
                Some("execution timed out".to_owned()),
                Some("traceback".to_owned()),
                Some(logs.clone()),
                now,
            )
            .unwrap();

        assert_eq!(completed.state, ExecJobState::TimedOut);
        assert!(completed.source.is_empty());
        assert_eq!(completed.result, None);
        assert_eq!(completed.logs, Some(logs));
        assert_eq!(
            completed.runtime_error.as_deref(),
            Some("execution timed out")
        );
        assert_eq!(completed.traceback.as_deref(), Some("traceback"));
    }

    #[test]
    fn duplicate_completion_is_rejected() {
        let store = test_store();
        let now = Instant::now();
        let job = submit_at(&store, "duplicate.lua", "return nil", now);
        store.claim_next_at(now).unwrap();
        store
            .complete_success_at(job.id, Some(ExecValue::Nil), None, now)
            .unwrap();

        assert_eq!(
            store.complete_failure_at(job.id, None, None, None, now),
            Err(ExecJobStoreError::DuplicateCompletion {
                id: job.id,
                state: ExecJobState::Succeeded,
            })
        );
    }

    #[test]
    fn unknown_job_is_rejected() {
        let store = test_store();
        let unknown_id = Uuid::new_v4();

        assert_eq!(
            store.get(unknown_id),
            Err(ExecJobStoreError::UnknownJob { id: unknown_id })
        );
        assert_eq!(
            store.complete_success(unknown_id, None, None),
            Err(ExecJobStoreError::UnknownJob { id: unknown_id })
        );
    }

    #[test]
    fn cleanup_times_out_pending_jobs() {
        let store = test_store();
        let now = Instant::now();
        let job = submit_at(&store, "pending.lua", "return nil", now);

        let stats = store.cleanup_expired_at(now + TEST_CLAIM_TIMEOUT);

        assert_eq!(stats.timed_out_pending, 1);
        assert_eq!(stats.timed_out_claimed, 0);
        assert_eq!(store.get(job.id).unwrap().state, ExecJobState::TimedOut);
        assert_eq!(store.claim_next_at(now + TEST_CLAIM_TIMEOUT), None);
    }

    #[test]
    fn cleanup_times_out_claimed_jobs() {
        let store = test_store();
        let now = Instant::now();
        let job = submit_at(&store, "claimed.lua", "return nil", now);
        let claim_time = now + Duration::from_secs(1);
        store.claim_next_at(claim_time).unwrap();

        let stats = store.cleanup_expired_at(claim_time + TEST_EXECUTION_TIMEOUT);

        assert_eq!(stats.timed_out_pending, 0);
        assert_eq!(stats.timed_out_claimed, 1);
        let timed_out = store.get(job.id).unwrap();
        assert_eq!(timed_out.state, ExecJobState::TimedOut);
        assert_eq!(
            timed_out.execution_deadline,
            Some(claim_time + TEST_EXECUTION_TIMEOUT)
        );
    }

    #[test]
    fn cleanup_removes_old_terminal_jobs() {
        let store = test_store();
        let now = Instant::now();
        let job = submit_at(&store, "old.lua", "return nil", now);
        store.claim_next_at(now).unwrap();
        store.complete_success_at(job.id, None, None, now).unwrap();

        assert_eq!(
            store.cleanup_expired_at(now + TEST_TERMINAL_RETENTION - Duration::from_nanos(1)),
            ExecCleanupStats::default()
        );
        assert!(store.get(job.id).is_ok());

        let stats = store.cleanup_expired_at(now + TEST_TERMINAL_RETENTION);
        assert_eq!(stats.removed_terminal, 1);
        assert_eq!(
            store.get(job.id),
            Err(ExecJobStoreError::UnknownJob { id: job.id })
        );
    }

    #[test]
    fn pending_queue_limit_is_enforced() {
        let store = test_store();
        let now = Instant::now();

        for index in 0..MAX_PENDING_JOBS {
            submit_at(&store, &format!("{index}.lua"), "return nil", now);
        }

        assert_eq!(
            store.submit_at(
                "overflow.lua".to_owned(),
                "return nil".to_owned(),
                SystemTime::UNIX_EPOCH,
                now,
            ),
            Err(ExecJobStoreError::PendingQueueFull {
                limit: MAX_PENDING_JOBS,
            })
        );

        store.claim_next_at(now).unwrap();
        assert!(store
            .submit_at(
                "replacement.lua".to_owned(),
                "return nil".to_owned(),
                SystemTime::UNIX_EPOCH,
                now,
            )
            .is_ok());
    }

    #[test]
    fn source_size_limit_is_enforced() {
        let store = test_store();

        assert!(store
            .submit("maximum.lua", "x".repeat(MAX_SOURCE_SIZE_BYTES))
            .is_ok());
        assert_eq!(
            store.submit("too-large.lua", "x".repeat(MAX_SOURCE_SIZE_BYTES + 1)),
            Err(ExecJobStoreError::SourceTooLarge {
                size: MAX_SOURCE_SIZE_BYTES + 1,
                limit: MAX_SOURCE_SIZE_BYTES,
            })
        );
    }

    #[test]
    fn terminal_job_limit_evicts_the_oldest_job() {
        let store = test_store();
        let now = Instant::now();
        let mut completed_ids = Vec::new();

        for index in 0..=MAX_RETAINED_TERMINAL_JOBS {
            let job = submit_at(&store, &format!("{index}.lua"), "return nil", now);
            store.claim_next_at(now).unwrap();
            store.complete_success_at(job.id, None, None, now).unwrap();
            completed_ids.push(job.id);
        }

        assert_eq!(
            store.get(completed_ids[0]),
            Err(ExecJobStoreError::UnknownJob {
                id: completed_ids[0],
            })
        );
        for id in &completed_ids[1..] {
            assert!(store.get(*id).is_ok());
        }
    }
}
