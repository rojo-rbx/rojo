use std::{
    fmt,
    str::FromStr,
    sync::Mutex,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::SessionId;

pub const AUTOMATION_HANDLER_VERSION: u32 = 2;
pub const AUTOMATION_SESSION_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PluginSessionId(Uuid);

impl PluginSessionId {
    #[cfg(test)]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl fmt::Display for PluginSessionId {
    fn fmt(&self, writer: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(writer, "{}", self.0)
    }
}

impl FromStr for PluginSessionId {
    type Err = uuid::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(value).map(Self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StudioMode {
    Edit,
    Play,
    Run,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutomationSessionUpdate {
    pub plugin_session_id: PluginSessionId,
    pub server_session_id: SessionId,
    pub studio_mode: StudioMode,
    pub exec_handler_available: bool,
    pub automation_handler_version: u32,
    pub plugin_version: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AutomationRegistration {
    Registered,
    Refreshed,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutomationPluginStatus {
    pub plugin_session_id: PluginSessionId,
    pub server_session_id: SessionId,
    pub connected: bool,
    pub last_seen_at_ms: u64,
    pub studio_mode: StudioMode,
    pub exec_handler_available: bool,
    pub automation_handler_version: u32,
    pub plugin_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutomationStatusSnapshot {
    pub plugin: Option<AutomationPluginStatus>,
    pub duplicate_session_detected: bool,
    pub exec_claimed_by_plugin_session_id: Option<PluginSessionId>,
}

#[derive(Debug)]
pub struct AutomationStatusStore {
    inner: Mutex<AutomationStatusInner>,
    session_timeout: Duration,
}

#[derive(Debug, Default)]
struct AutomationStatusInner {
    active: Option<StoredAutomationPluginStatus>,
    duplicate_last_seen: Option<Instant>,
    exec_claimed_by_plugin_session_id: Option<PluginSessionId>,
}

#[derive(Debug, Clone)]
struct StoredAutomationPluginStatus {
    update: AutomationSessionUpdate,
    last_seen: Instant,
    last_seen_at: SystemTime,
}

impl AutomationStatusStore {
    pub fn new() -> Self {
        Self::with_timeout(AUTOMATION_SESSION_TIMEOUT)
    }

    pub fn refresh(&self, update: AutomationSessionUpdate) -> AutomationRegistration {
        self.refresh_at(update, Instant::now(), SystemTime::now())
    }

    pub fn snapshot(&self) -> AutomationStatusSnapshot {
        self.snapshot_at(Instant::now())
    }

    pub fn note_exec_claim(&self, plugin_session_id: PluginSessionId) {
        let mut inner = self.inner.lock().unwrap();
        inner.exec_claimed_by_plugin_session_id = Some(plugin_session_id);
    }

    pub fn note_exec_completion(&self, plugin_session_id: PluginSessionId) {
        let mut inner = self.inner.lock().unwrap();
        if inner.exec_claimed_by_plugin_session_id == Some(plugin_session_id) {
            inner.exec_claimed_by_plugin_session_id = None;
        }
    }

    pub fn clear_exec_claim_if_idle(&self, has_claimed_job: bool) {
        if !has_claimed_job {
            self.inner.lock().unwrap().exec_claimed_by_plugin_session_id = None;
        }
    }

    fn with_timeout(session_timeout: Duration) -> Self {
        Self {
            inner: Mutex::new(AutomationStatusInner::default()),
            session_timeout,
        }
    }

    fn refresh_at(
        &self,
        mut update: AutomationSessionUpdate,
        now: Instant,
        wall_clock: SystemTime,
    ) -> AutomationRegistration {
        let mut inner = self.inner.lock().unwrap();
        let active_is_current = inner.active.as_ref().is_some_and(|active| {
            now.saturating_duration_since(active.last_seen) < self.session_timeout
        });

        if active_is_current {
            let active = inner.active.as_mut().unwrap();
            if active.update.plugin_session_id != update.plugin_session_id {
                inner.duplicate_last_seen = Some(now);
                return AutomationRegistration::Conflict;
            }

            if update.plugin_version.is_none() {
                update.plugin_version = active.update.plugin_version.clone();
            }
            active.update = update;
            active.last_seen = now;
            active.last_seen_at = wall_clock;
            return AutomationRegistration::Refreshed;
        }

        inner.active = Some(StoredAutomationPluginStatus {
            update,
            last_seen: now,
            last_seen_at: wall_clock,
        });
        inner.duplicate_last_seen = None;
        AutomationRegistration::Registered
    }

    fn snapshot_at(&self, now: Instant) -> AutomationStatusSnapshot {
        let inner = self.inner.lock().unwrap();
        let plugin = inner.active.as_ref().map(|active| AutomationPluginStatus {
            plugin_session_id: active.update.plugin_session_id,
            server_session_id: active.update.server_session_id,
            connected: now.saturating_duration_since(active.last_seen) < self.session_timeout,
            last_seen_at_ms: active
                .last_seen_at
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                .try_into()
                .unwrap_or(u64::MAX),
            studio_mode: active.update.studio_mode,
            exec_handler_available: active.update.exec_handler_available,
            automation_handler_version: active.update.automation_handler_version,
            plugin_version: active.update.plugin_version.clone(),
        });
        let duplicate_session_detected = inner.duplicate_last_seen.is_some_and(|last_seen| {
            now.saturating_duration_since(last_seen) < self.session_timeout
        });

        AutomationStatusSnapshot {
            plugin,
            duplicate_session_detected,
            exec_claimed_by_plugin_session_id: inner.exec_claimed_by_plugin_session_id,
        }
    }
}

impl Default for AutomationStatusStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn update(plugin_session_id: PluginSessionId, mode: StudioMode) -> AutomationSessionUpdate {
        AutomationSessionUpdate {
            plugin_session_id,
            server_session_id: SessionId::new(),
            studio_mode: mode,
            exec_handler_available: true,
            automation_handler_version: AUTOMATION_HANDLER_VERSION,
            plugin_version: Some("7.6.1".to_owned()),
        }
    }

    #[test]
    fn reports_no_plugin_before_registration() {
        let store = AutomationStatusStore::new();
        let status = store.snapshot();
        assert_eq!(status.plugin, None);
        assert!(!status.duplicate_session_detected);
    }

    #[test]
    fn registers_and_refreshes_the_same_session() {
        let store = AutomationStatusStore::new();
        let plugin_session_id = PluginSessionId::new();
        assert_eq!(
            store.refresh(update(plugin_session_id, StudioMode::Edit)),
            AutomationRegistration::Registered
        );
        assert_eq!(
            store.refresh(update(plugin_session_id, StudioMode::Play)),
            AutomationRegistration::Refreshed
        );
        assert_eq!(
            store.snapshot().plugin.unwrap().studio_mode,
            StudioMode::Play
        );
    }

    #[test]
    fn detects_a_second_active_session() {
        let store = AutomationStatusStore::new();
        let first = PluginSessionId::new();
        let second = PluginSessionId::new();
        store.refresh(update(first, StudioMode::Edit));
        assert_eq!(
            store.refresh(update(second, StudioMode::Edit)),
            AutomationRegistration::Conflict
        );
        let status = store.snapshot();
        assert_eq!(status.plugin.unwrap().plugin_session_id, first);
        assert!(status.duplicate_session_detected);
    }

    #[test]
    fn a_stale_session_can_be_replaced() {
        let timeout = Duration::from_secs(10);
        let store = AutomationStatusStore::with_timeout(timeout);
        let started = Instant::now();
        let first = PluginSessionId::new();
        let second = PluginSessionId::new();
        store.refresh_at(update(first, StudioMode::Edit), started, UNIX_EPOCH);

        let stale = store.snapshot_at(started + timeout);
        assert!(!stale.plugin.unwrap().connected);
        assert_eq!(
            store.refresh_at(
                update(second, StudioMode::Run),
                started + timeout,
                UNIX_EPOCH + Duration::from_secs(1),
            ),
            AutomationRegistration::Registered
        );
        assert_eq!(store.snapshot().plugin.unwrap().plugin_session_id, second);
    }

    #[test]
    fn serializes_all_studio_modes_as_camel_case_strings() {
        for (mode, expected) in [
            (StudioMode::Edit, "\"edit\""),
            (StudioMode::Play, "\"play\""),
            (StudioMode::Run, "\"run\""),
            (StudioMode::Unknown, "\"unknown\""),
        ] {
            assert_eq!(serde_json::to_string(&mode).unwrap(), expected);
        }
    }
}
