//! Defines Rojo's HTTP API, all under /api. These endpoints generally return
//! JSON.

use std::{collections::HashMap, fs, net::SocketAddr, path::PathBuf, str::FromStr, sync::Arc};

use futures::{sink::SinkExt, stream::StreamExt};
use hyper::{
    body::{self, HttpBody},
    header::{HeaderValue, CACHE_CONTROL},
    Body, Method, Request, Response, StatusCode,
};
use hyper_tungstenite::{is_upgrade_request, tungstenite::Message, upgrade, HyperWebsocket};
use opener::OpenError;
use rbx_dom_weak::{
    types::{Ref, Variant},
    InstanceBuilder, UstrMap, WeakDom,
};
use uuid::Uuid;

use crate::{
    automation::{
        AutomationJobStoreError, AutomationRequest, MAX_AUTOMATION_REQUEST_BODY_BYTES,
        MAX_AUTOMATION_RESULT_BODY_BYTES,
    },
    automation_status::{
        AutomationRegistration, AutomationSessionUpdate, PluginSessionId,
        AUTOMATION_HANDLER_VERSION,
    },
    exec::{ExecJobStoreError, MAX_SOURCE_SIZE_BYTES},
    serve_session::ServeSession,
    snapshot::{InstanceWithMeta, PatchSet, PatchUpdate},
    web::{
        interface::{
            AutomationHeartbeatRequest, AutomationHeartbeatResponse, AutomationJobClaimResponse,
            AutomationJobCompletion, AutomationJobCompletionRequest, AutomationJobResponse,
            AutomationPluginStatusResponse, AutomationQueueStatusResponse,
            AutomationStatusResponse, ErrorResponse, ExecJobClaimResponse,
            ExecJobCompletionEnvelope, ExecJobCompletionRequest, ExecJobResponse,
            ExecJobSubmissionRequest, ExecLog, Instance, MessagesPacket, OpenResponse,
            ReadResponse, ServerInfoResponse, SocketPacket, SocketPacketBody, SocketPacketType,
            StudioMode, SubscribeMessage, WriteRequest, WriteResponse, PROTOCOL_VERSION,
            SERVER_VERSION,
        },
        origin::canonical,
        util::{deserialize_msgpack, msgpack, msgpack_ok, response, serialize_msgpack},
    },
    web_api::{
        InstanceUpdate, RefPatchRequest, RefPatchResponse, SerializeRequest, SerializeResponse,
    },
};

const EXEC_SUBMISSION_BODY_LIMIT_BYTES: usize = MAX_SOURCE_SIZE_BYTES + 64 * 1024;
const EXEC_COMPLETION_BODY_LIMIT_BYTES: usize = 256 * 1024;
const AUTOMATION_STATUS_BODY_LIMIT_BYTES: usize = 16 * 1024;

enum ExecRoute {
    Submit,
    ClaimNext,
    Status(String),
    Complete(String),
}

enum AutomationRoute {
    Submit,
    ClaimNext,
    Status(String),
    Complete(String),
}

impl AutomationRoute {
    fn parse(method: &Method, path: &str) -> Option<Self> {
        match (method, path) {
            (&Method::POST, "/api/automation/jobs") => return Some(Self::Submit),
            (&Method::GET, "/api/automation/jobs/next") => return Some(Self::ClaimNext),
            _ => {}
        }
        let remainder = path.strip_prefix("/api/automation/jobs/")?;
        let mut segments = remainder.split('/');
        let id = segments.next()?;
        let suffix = segments.next();
        if id.is_empty() || segments.next().is_some() {
            return None;
        }
        match (method, suffix) {
            (&Method::GET, None) => Some(Self::Status(id.to_owned())),
            (&Method::POST, Some("complete")) => Some(Self::Complete(id.to_owned())),
            _ => None,
        }
    }
}

impl ExecRoute {
    fn parse(method: &Method, path: &str) -> Option<Self> {
        match (method, path) {
            (&Method::POST, "/api/exec/jobs") => return Some(Self::Submit),
            (&Method::GET, "/api/exec/jobs/next") => return Some(Self::ClaimNext),
            _ => {}
        }

        let remainder = path.strip_prefix("/api/exec/jobs/")?;
        let mut segments = remainder.split('/');
        let id = segments.next()?;
        let suffix = segments.next();

        if id.is_empty() || segments.next().is_some() {
            return None;
        }

        match (method, suffix) {
            (&Method::GET, None) => Some(Self::Status(id.to_owned())),
            (&Method::POST, Some("complete")) => Some(Self::Complete(id.to_owned())),
            _ => None,
        }
    }
}

pub async fn call(
    serve_session: Arc<ServeSession>,
    remote_addr: SocketAddr,
    mut request: Request<Body>,
) -> Response<Body> {
    let service = ApiService::new(serve_session, remote_addr);

    if let Some(route) = ExecRoute::parse(request.method(), request.uri().path()) {
        if let Some(response) = service.reject_non_local("/api/exec") {
            return response;
        }

        service.serve_session.exec_job_store().cleanup_expired();

        return match route {
            ExecRoute::Submit => service.handle_api_exec_submit(request).await,
            ExecRoute::ClaimNext => service.handle_api_exec_claim_next(&request),
            ExecRoute::Status(id) => service.handle_api_exec_status(&id),
            ExecRoute::Complete(id) => service.handle_api_exec_complete(&id, request).await,
        };
    }

    if let Some(route) = AutomationRoute::parse(request.method(), request.uri().path()) {
        if let Some(response) = service.reject_non_local("/api/automation/jobs") {
            return response;
        }
        service
            .serve_session
            .automation_job_store()
            .cleanup_expired();
        return match route {
            AutomationRoute::Submit => service.handle_api_automation_submit(request).await,
            AutomationRoute::ClaimNext => service.handle_api_automation_claim_next(&request),
            AutomationRoute::Status(id) => service.handle_api_automation_job_status(&id),
            AutomationRoute::Complete(id) => {
                service.handle_api_automation_complete(&id, request).await
            }
        };
    }

    match (request.method(), request.uri().path()) {
        (&Method::GET, "/api/rojo") => service.handle_api_rojo().await,
        (&Method::GET, "/api/automation/status") => {
            if let Some(response) = service.reject_non_local("/api/automation/status") {
                response
            } else {
                service.handle_api_automation_status()
            }
        }
        (&Method::POST, "/api/automation/status") => {
            if let Some(response) = service.reject_non_local("/api/automation/status") {
                response
            } else {
                service.handle_api_automation_heartbeat(request).await
            }
        }
        (&Method::GET, path) if path.starts_with("/api/read/") => {
            service.handle_api_read(request).await
        }
        (&Method::GET, path) if path.starts_with("/api/socket/") => {
            if is_upgrade_request(&request) {
                service.handle_api_socket(&mut request).await
            } else {
                msgpack(
                    ErrorResponse::bad_request(
                        "/api/socket must be called as a websocket upgrade request",
                    ),
                    StatusCode::BAD_REQUEST,
                )
            }
        }
        (&Method::POST, "/api/serialize") => service.handle_api_serialize(request).await,
        (&Method::POST, "/api/ref-patch") => service.handle_api_ref_patch(request).await,

        (&Method::POST, path) if path.starts_with("/api/open/") => {
            service.handle_api_open(request).await
        }
        (&Method::POST, "/api/write") => service.handle_api_write(request).await,

        (_method, path) => msgpack(
            ErrorResponse::not_found(format!("Route not found: {}", path)),
            StatusCode::NOT_FOUND,
        ),
    }
}

pub struct ApiService {
    serve_session: Arc<ServeSession>,
    remote_addr: SocketAddr,
}

impl ApiService {
    pub fn new(serve_session: Arc<ServeSession>, remote_addr: SocketAddr) -> Self {
        ApiService {
            serve_session,
            remote_addr,
        }
    }

    fn reject_non_local(&self, route: &str) -> Option<Response<Body>> {
        if canonical(self.remote_addr.ip()).is_loopback() {
            return None;
        }

        Some(msgpack(
            ErrorResponse::forbidden(format!("{route} is only available to local clients")),
            StatusCode::FORBIDDEN,
        ))
    }

    async fn handle_api_exec_submit(&self, request: Request<Body>) -> Response<Body> {
        let body = match read_bounded_body(
            request.into_body(),
            EXEC_SUBMISSION_BODY_LIMIT_BYTES,
            "Exec",
        )
        .await
        {
            Ok(body) => body,
            Err(response) => return response,
        };
        let request: ExecJobSubmissionRequest = match deserialize_msgpack(&body) {
            Ok(request) => request,
            Err(err) => {
                return msgpack(
                    ErrorResponse::bad_request(format!("Invalid body: {err}")),
                    StatusCode::BAD_REQUEST,
                );
            }
        };

        if request.script_name.is_empty() {
            return msgpack(
                ErrorResponse::bad_request("Exec script name must not be empty"),
                StatusCode::BAD_REQUEST,
            );
        }

        if request.script_name.chars().any(char::is_control) {
            return msgpack(
                ErrorResponse::bad_request("Exec script name must not contain control characters"),
                StatusCode::BAD_REQUEST,
            );
        }

        match self
            .serve_session
            .exec_job_store()
            .submit(request.script_name, request.source)
        {
            Ok(job) => msgpack(ExecJobResponse::from(job), StatusCode::CREATED),
            Err(err) => exec_store_error(err),
        }
    }

    fn handle_api_exec_claim_next(&self, request: &Request<Body>) -> Response<Body> {
        let plugin_session = match parse_exec_session_query(request.uri().query()) {
            Ok(plugin_session) => plugin_session,
            Err(response) => return *response,
        };

        if let Some((plugin_session_id, studio_mode)) = plugin_session {
            if let Err(response) = self.refresh_automation_session(plugin_session_id, studio_mode) {
                return *response;
            }
        }

        match self.serve_session.exec_job_store().claim_next() {
            Some(job) => {
                if let Some((plugin_session_id, _)) = plugin_session {
                    self.serve_session
                        .automation_status()
                        .note_exec_claim(plugin_session_id);
                }
                no_store(msgpack_ok(ExecJobClaimResponse::from(job)))
            }
            None => no_store(response(
                StatusCode::NO_CONTENT,
                "application/msgpack",
                Body::empty(),
            )),
        }
    }

    fn handle_api_exec_status(&self, id: &str) -> Response<Body> {
        let id = match Uuid::parse_str(id) {
            Ok(id) => id,
            Err(err) => {
                return msgpack(
                    ErrorResponse::bad_request(format!("Malformed exec job ID: {err}")),
                    StatusCode::BAD_REQUEST,
                );
            }
        };

        match self.serve_session.exec_job_store().get(id) {
            Ok(job) => no_store(msgpack_ok(ExecJobResponse::from(job))),
            Err(err) => exec_store_error(err),
        }
    }

    async fn handle_api_exec_complete(&self, id: &str, request: Request<Body>) -> Response<Body> {
        let id = match Uuid::parse_str(id) {
            Ok(id) => id,
            Err(err) => {
                return msgpack(
                    ErrorResponse::bad_request(format!("Malformed exec job ID: {err}")),
                    StatusCode::BAD_REQUEST,
                );
            }
        };

        let body = match read_bounded_body(
            request.into_body(),
            EXEC_COMPLETION_BODY_LIMIT_BYTES,
            "Exec",
        )
        .await
        {
            Ok(body) => body,
            Err(response) => return response,
        };
        let request: ExecJobCompletionEnvelope = match deserialize_msgpack(&body) {
            Ok(request) => request,
            Err(err) => {
                return msgpack(
                    ErrorResponse::bad_request(format!("Invalid body: {err}")),
                    StatusCode::BAD_REQUEST,
                );
            }
        };

        let plugin_session = match parse_exec_completion_session(
            request.plugin_session_id.as_deref(),
            request.studio_mode,
        ) {
            Ok(plugin_session) => plugin_session,
            Err(response) => return *response,
        };
        if let Some((plugin_session_id, studio_mode)) = plugin_session {
            if let Err(response) = self.refresh_automation_session(plugin_session_id, studio_mode) {
                return *response;
            }
        }

        let store = self.serve_session.exec_job_store();
        let result = match request.completion {
            ExecJobCompletionRequest::Success { result, logs } => {
                store.complete_success(id, result.map(Into::into), into_stored_logs(logs))
            }
            ExecJobCompletionRequest::CompileFailure {
                error,
                traceback,
                logs,
            }
            | ExecJobCompletionRequest::RuntimeFailure {
                error,
                traceback,
                logs,
            }
            | ExecJobCompletionRequest::Rejected {
                error,
                traceback,
                logs,
            } => store.complete_failure(id, Some(error), traceback, into_stored_logs(logs)),
            ExecJobCompletionRequest::Timeout {
                error,
                traceback,
                logs,
            } => store.complete_timeout(id, Some(error), traceback, into_stored_logs(logs)),
        };

        match result {
            Ok(job) => {
                if let Some((plugin_session_id, _)) = plugin_session {
                    self.serve_session
                        .automation_status()
                        .note_exec_completion(plugin_session_id);
                }
                no_store(msgpack_ok(ExecJobResponse::from(job)))
            }
            Err(err) => exec_store_error(err),
        }
    }

    async fn handle_api_automation_heartbeat(&self, request: Request<Body>) -> Response<Body> {
        let body = match read_bounded_body(
            request.into_body(),
            AUTOMATION_STATUS_BODY_LIMIT_BYTES,
            "Automation status",
        )
        .await
        {
            Ok(body) => body,
            Err(response) => return response,
        };
        let request: AutomationHeartbeatRequest = match deserialize_msgpack(&body) {
            Ok(request) => request,
            Err(err) => {
                return msgpack(
                    ErrorResponse::bad_request(format!("Invalid body: {err}")),
                    StatusCode::BAD_REQUEST,
                );
            }
        };

        if request.server_session_id != self.serve_session.session_id() {
            return msgpack(
                ErrorResponse::conflict("Automation heartbeat targets a different server session"),
                StatusCode::CONFLICT,
            );
        }

        let plugin_session_id = match request.plugin_session_id.parse::<PluginSessionId>() {
            Ok(plugin_session_id) => plugin_session_id,
            Err(err) => {
                return msgpack(
                    ErrorResponse::bad_request(format!("Malformed plugin session ID: {err}")),
                    StatusCode::BAD_REQUEST,
                );
            }
        };
        let registration =
            self.serve_session
                .automation_status()
                .refresh(AutomationSessionUpdate {
                    plugin_session_id,
                    server_session_id: request.server_session_id,
                    studio_mode: request.studio_mode,
                    exec_handler_available: request.exec_handler_available,
                    automation_handler_version: request.automation_handler_version,
                    plugin_version: request.plugin_version,
                });
        let active_plugin_session_id = self
            .serve_session
            .automation_status()
            .snapshot()
            .plugin
            .map(|plugin| plugin.plugin_session_id.to_string());

        no_store(msgpack_ok(AutomationHeartbeatResponse {
            registration,
            active_plugin_session_id,
        }))
    }

    async fn handle_api_automation_submit(&self, request: Request<Body>) -> Response<Body> {
        let body = match read_bounded_body(
            request.into_body(),
            MAX_AUTOMATION_REQUEST_BODY_BYTES,
            "Automation job",
        )
        .await
        {
            Ok(body) => body,
            Err(response) => return response,
        };
        let request: AutomationRequest = match deserialize_msgpack(&body) {
            Ok(request) => request,
            Err(err) => {
                return msgpack(
                    ErrorResponse::bad_request(format!("Invalid automation request: {err}")),
                    StatusCode::BAD_REQUEST,
                );
            }
        };
        match self.serve_session.automation_job_store().submit(request) {
            Ok(job) => no_store(msgpack(
                AutomationJobResponse::from(job),
                StatusCode::CREATED,
            )),
            Err(error) => automation_store_error(error),
        }
    }

    fn handle_api_automation_claim_next(&self, request: &Request<Body>) -> Response<Body> {
        let (plugin_session_id, studio_mode) = match parse_exec_session_query(request.uri().query())
        {
            Ok(Some(metadata)) => metadata,
            Ok(None) => {
                return msgpack(
                    ErrorResponse::bad_request(
                        "Automation claim requires pluginSessionId and studioMode",
                    ),
                    StatusCode::BAD_REQUEST,
                );
            }
            Err(response) => return *response,
        };
        if studio_mode != StudioMode::Edit {
            return msgpack(
                ErrorResponse::conflict("Typed automation jobs may only be claimed in edit mode"),
                StatusCode::CONFLICT,
            );
        }
        if let Err(response) = self.refresh_typed_automation_session(plugin_session_id, studio_mode)
        {
            return *response;
        }
        match self
            .serve_session
            .automation_job_store()
            .claim_next(plugin_session_id)
        {
            Some(job) => no_store(msgpack_ok(AutomationJobClaimResponse::from(job))),
            None => no_store(response(
                StatusCode::NO_CONTENT,
                "application/msgpack",
                Body::empty(),
            )),
        }
    }

    fn handle_api_automation_job_status(&self, id: &str) -> Response<Body> {
        let id = match Uuid::parse_str(id) {
            Ok(id) => id,
            Err(err) => {
                return msgpack(
                    ErrorResponse::bad_request(format!("Malformed automation job ID: {err}")),
                    StatusCode::BAD_REQUEST,
                );
            }
        };
        match self.serve_session.automation_job_store().get(id) {
            Ok(job) => no_store(msgpack_ok(AutomationJobResponse::from(job))),
            Err(error) => automation_store_error(error),
        }
    }

    async fn handle_api_automation_complete(
        &self,
        id: &str,
        request: Request<Body>,
    ) -> Response<Body> {
        let id = match Uuid::parse_str(id) {
            Ok(id) => id,
            Err(err) => {
                return msgpack(
                    ErrorResponse::bad_request(format!("Malformed automation job ID: {err}")),
                    StatusCode::BAD_REQUEST,
                );
            }
        };
        let body = match read_bounded_body(
            request.into_body(),
            MAX_AUTOMATION_RESULT_BODY_BYTES,
            "Automation completion",
        )
        .await
        {
            Ok(body) => body,
            Err(response) => return response,
        };
        let request: AutomationJobCompletionRequest = match deserialize_msgpack(&body) {
            Ok(request) => request,
            Err(err) => {
                return msgpack(
                    ErrorResponse::bad_request(format!("Invalid automation completion: {err}")),
                    StatusCode::BAD_REQUEST,
                );
            }
        };
        let plugin_session_id = match request.plugin_session_id.parse::<PluginSessionId>() {
            Ok(id) => id,
            Err(err) => {
                return msgpack(
                    ErrorResponse::bad_request(format!("Malformed plugin session ID: {err}")),
                    StatusCode::BAD_REQUEST,
                );
            }
        };
        if let Err(response) =
            self.refresh_typed_automation_session(plugin_session_id, request.studio_mode)
        {
            return *response;
        }
        let store = self.serve_session.automation_job_store();
        let completed = match request.completion {
            AutomationJobCompletion::Success { result } => {
                store.complete_success(id, plugin_session_id, *result)
            }
            AutomationJobCompletion::Failure { error } => {
                store.complete_failure(id, plugin_session_id, error)
            }
        };
        match completed {
            Ok(job) => no_store(msgpack_ok(AutomationJobResponse::from(job))),
            Err(error) => automation_store_error(error),
        }
    }

    fn handle_api_automation_status(&self) -> Response<Body> {
        self.serve_session.exec_job_store().cleanup_expired();
        let queue_counts = self.serve_session.exec_job_store().queue_counts();
        self.serve_session.automation_job_store().cleanup_expired();
        let automation_queue_counts = self.serve_session.automation_job_store().queue_counts();
        self.serve_session
            .automation_status()
            .clear_exec_claim_if_idle(queue_counts.claimed != 0);
        let status = self.serve_session.automation_status().snapshot();
        let automation_available = status.plugin.as_ref().is_some_and(|plugin| {
            plugin.connected && plugin.automation_handler_version == AUTOMATION_HANDLER_VERSION
        });
        let exec_available = automation_available
            && status
                .plugin
                .as_ref()
                .is_some_and(|plugin| plugin.exec_handler_available);
        let plugin = status.plugin.map(|plugin| AutomationPluginStatusResponse {
            connected: plugin.connected,
            plugin_session_id: plugin.plugin_session_id.to_string(),
            studio_mode: plugin.studio_mode,
            plugin_version: plugin.plugin_version,
            automation_handler_version: plugin.automation_handler_version,
            last_seen_at_ms: plugin.last_seen_at_ms,
        });

        no_store(msgpack_ok(AutomationStatusResponse {
            server_session_id: self.serve_session.session_id(),
            server_version: SERVER_VERSION.to_owned(),
            protocol_version: PROTOCOL_VERSION,
            automation_handler_version: AUTOMATION_HANDLER_VERSION,
            automation_available,
            exec_available,
            typed_automation_available: automation_available,
            plugin,
            duplicate_session_detected: status.duplicate_session_detected,
            queues: AutomationQueueStatusResponse {
                exec_pending: queue_counts.pending,
                exec_claimed: queue_counts.claimed,
                exec_claimed_by_plugin_session_id: status
                    .exec_claimed_by_plugin_session_id
                    .map(|id| id.to_string()),
                automation_pending: automation_queue_counts.pending,
                automation_claimed: automation_queue_counts.claimed,
                automation_claimed_by_plugin_session_id: self
                    .serve_session
                    .automation_job_store()
                    .claimed_by()
                    .map(|id| id.to_string()),
            },
        }))
    }

    fn refresh_automation_session(
        &self,
        plugin_session_id: PluginSessionId,
        studio_mode: StudioMode,
    ) -> Result<(), Box<Response<Body>>> {
        let snapshot = self.serve_session.automation_status().snapshot();
        let existing = snapshot
            .plugin
            .filter(|plugin| plugin.plugin_session_id == plugin_session_id);
        let registration =
            self.serve_session
                .automation_status()
                .refresh(AutomationSessionUpdate {
                    plugin_session_id,
                    server_session_id: self.serve_session.session_id(),
                    studio_mode,
                    exec_handler_available: existing
                        .as_ref()
                        .is_none_or(|plugin| plugin.exec_handler_available),
                    automation_handler_version: existing
                        .as_ref()
                        .map_or(AUTOMATION_HANDLER_VERSION, |plugin| {
                            plugin.automation_handler_version
                        }),
                    plugin_version: None,
                });

        if registration == AutomationRegistration::Conflict {
            return Err(Box::new(msgpack(
                ErrorResponse::conflict(
                    "Another active Prism plugin session is already registered for automation",
                ),
                StatusCode::CONFLICT,
            )));
        }

        Ok(())
    }

    fn refresh_typed_automation_session(
        &self,
        plugin_session_id: PluginSessionId,
        studio_mode: StudioMode,
    ) -> Result<(), Box<Response<Body>>> {
        let snapshot = self.serve_session.automation_status().snapshot();
        let authoritative = snapshot.plugin.as_ref().is_some_and(|plugin| {
            plugin.connected
                && plugin.plugin_session_id == plugin_session_id
                && plugin.automation_handler_version == AUTOMATION_HANDLER_VERSION
        });
        if !authoritative {
            return Err(Box::new(msgpack(
                ErrorResponse::conflict(
                    "The claimant is not the active compatible Prism automation plugin session",
                ),
                StatusCode::CONFLICT,
            )));
        }
        self.refresh_automation_session(plugin_session_id, studio_mode)
    }

    /// Get a summary of information about the server
    async fn handle_api_rojo(&self) -> Response<Body> {
        let tree = self.serve_session.tree();
        let root_instance_id = tree.get_root_id();

        msgpack_ok(&ServerInfoResponse {
            server_version: SERVER_VERSION.to_owned(),
            protocol_version: PROTOCOL_VERSION,
            session_id: self.serve_session.session_id(),
            project_name: self.serve_session.project_name().to_owned(),
            expected_place_ids: self.serve_session.serve_place_ids().cloned(),
            unexpected_place_ids: self.serve_session.blocked_place_ids().cloned(),
            place_id: self.serve_session.place_id(),
            game_id: self.serve_session.game_id(),
            root_instance_id,
        })
    }

    /// Handle WebSocket upgrade for real-time message streaming
    async fn handle_api_socket(&self, request: &mut Request<Body>) -> Response<Body> {
        let argument = &request.uri().path()["/api/socket/".len()..];
        let input_cursor: u32 = match argument.parse() {
            Ok(v) => v,
            Err(err) => {
                return msgpack(
                    ErrorResponse::bad_request(format!("Malformed message cursor: {}", err)),
                    StatusCode::BAD_REQUEST,
                );
            }
        };

        // Upgrade the connection to WebSocket
        let (response, websocket) = match upgrade(request, None) {
            Ok(result) => result,
            Err(err) => {
                return msgpack(
                    ErrorResponse::internal_error(format!("WebSocket upgrade failed: {}", err)),
                    StatusCode::INTERNAL_SERVER_ERROR,
                );
            }
        };

        let serve_session = Arc::clone(&self.serve_session);

        // Spawn a task to handle the WebSocket connection
        tokio::spawn(async move {
            if let Err(e) =
                handle_websocket_subscription(serve_session, websocket, input_cursor).await
            {
                log::error!("Error in websocket subscription: {}", e);
            }
        });

        response
    }

    async fn handle_api_write(&self, request: Request<Body>) -> Response<Body> {
        let session_id = self.serve_session.session_id();
        let tree_mutation_sender = self.serve_session.tree_mutation_sender();

        let body = body::to_bytes(request.into_body()).await.unwrap();

        let request: WriteRequest = match deserialize_msgpack(&body) {
            Ok(request) => request,
            Err(err) => {
                return msgpack(
                    ErrorResponse::bad_request(format!("Invalid body: {}", err)),
                    StatusCode::BAD_REQUEST,
                );
            }
        };

        if request.session_id != session_id {
            return msgpack(
                ErrorResponse::bad_request("Wrong session ID"),
                StatusCode::BAD_REQUEST,
            );
        }

        let updated_instances = request
            .updated
            .into_iter()
            .map(|update| PatchUpdate {
                id: update.id,
                changed_class_name: update.changed_class_name,
                changed_name: update.changed_name,
                changed_properties: update.changed_properties,
                changed_metadata: None,
            })
            .collect();

        tree_mutation_sender
            .send(PatchSet {
                removed_instances: Vec::new(),
                added_instances: Vec::new(),
                updated_instances,
            })
            .unwrap();

        msgpack_ok(WriteResponse { session_id })
    }

    async fn handle_api_read(&self, request: Request<Body>) -> Response<Body> {
        let argument = &request.uri().path()["/api/read/".len()..];
        let requested_ids: Result<Vec<Ref>, _> = argument.split(',').map(Ref::from_str).collect();

        let requested_ids = match requested_ids {
            Ok(ids) => ids,
            Err(_) => {
                return msgpack(
                    ErrorResponse::bad_request("Malformed ID list"),
                    StatusCode::BAD_REQUEST,
                );
            }
        };

        let message_queue = self.serve_session.message_queue();
        let message_cursor = message_queue.cursor();

        let tree = self.serve_session.tree();

        let mut instances = HashMap::new();

        for id in requested_ids {
            if let Some(instance) = tree.get_instance(id) {
                instances.insert(id, Instance::from_rojo_instance(instance));

                for descendant in tree.descendants(id) {
                    instances.insert(descendant.id(), Instance::from_rojo_instance(descendant));
                }
            }
        }

        msgpack_ok(ReadResponse {
            session_id: self.serve_session.session_id(),
            message_cursor,
            instances,
        })
    }

    /// Accepts a list of IDs and returns them serialized as a binary model.
    /// The model is sent in a schema that causes Roblox to deserialize it as
    /// a Luau `buffer`.
    ///
    /// The returned model is a folder that contains ObjectValues with names
    /// that correspond to the requested Instances. These values have their
    /// `Value` property set to point to the requested Instance.
    async fn handle_api_serialize(&self, request: Request<Body>) -> Response<Body> {
        let session_id = self.serve_session.session_id();
        let body = body::to_bytes(request.into_body()).await.unwrap();

        let request: SerializeRequest = match deserialize_msgpack(&body) {
            Ok(request) => request,
            Err(err) => {
                return msgpack(
                    ErrorResponse::bad_request(format!("Invalid body: {}", err)),
                    StatusCode::BAD_REQUEST,
                );
            }
        };

        if request.session_id != session_id {
            return msgpack(
                ErrorResponse::bad_request("Wrong session ID"),
                StatusCode::BAD_REQUEST,
            );
        }

        let mut response_dom = WeakDom::new(InstanceBuilder::new("Folder"));

        let tree = self.serve_session.tree();
        for id in &request.ids {
            if let Some(instance) = tree.get_instance(*id) {
                let clone = response_dom.insert(
                    Ref::none(),
                    InstanceBuilder::new(instance.class_name())
                        .with_name(instance.name())
                        .with_properties(instance.properties().clone()),
                );
                let object_value = response_dom.insert(
                    response_dom.root_ref(),
                    InstanceBuilder::new("ObjectValue")
                        .with_name(id.to_string())
                        .with_property("Value", clone),
                );

                let mut child_ref = clone;
                if let Some(parent_class) = parent_requirements(&instance.class_name()) {
                    child_ref =
                        response_dom.insert(object_value, InstanceBuilder::new(parent_class));
                    response_dom.transfer_within(clone, child_ref);
                }

                response_dom.transfer_within(child_ref, object_value);
            } else {
                return msgpack(
                    ErrorResponse::bad_request(format!("provided id {id} is not in the tree")),
                    StatusCode::BAD_REQUEST,
                );
            }
        }
        drop(tree);

        let mut source = Vec::new();
        rbx_binary::to_writer(&mut source, &response_dom, &[response_dom.root_ref()]).unwrap();

        msgpack_ok(SerializeResponse {
            session_id: self.serve_session.session_id(),
            model_contents: source,
        })
    }

    /// Returns a list of all referent properties that point towards the
    /// provided IDs. Used because the plugin does not store a RojoTree,
    /// and referent properties need to be updated after the serialize
    /// endpoint is used.
    async fn handle_api_ref_patch(self, request: Request<Body>) -> Response<Body> {
        let session_id = self.serve_session.session_id();
        let body = body::to_bytes(request.into_body()).await.unwrap();

        let request: RefPatchRequest = match deserialize_msgpack(&body) {
            Ok(request) => request,
            Err(err) => {
                return msgpack(
                    ErrorResponse::bad_request(format!("Invalid body: {}", err)),
                    StatusCode::BAD_REQUEST,
                );
            }
        };

        if request.session_id != session_id {
            return msgpack(
                ErrorResponse::bad_request("Wrong session ID"),
                StatusCode::BAD_REQUEST,
            );
        }

        let mut instance_updates: HashMap<Ref, InstanceUpdate> = HashMap::new();

        let tree = self.serve_session.tree();
        for instance in tree.descendants(tree.get_root_id()) {
            for (prop_name, prop_value) in instance.properties() {
                let Variant::Ref(prop_value) = prop_value else {
                    continue;
                };
                if let Some(target_id) = request.ids.get(prop_value) {
                    let instance_id = instance.id();
                    let update =
                        instance_updates
                            .entry(instance_id)
                            .or_insert_with(|| InstanceUpdate {
                                id: instance_id,
                                changed_class_name: None,
                                changed_name: None,
                                changed_metadata: None,
                                changed_properties: UstrMap::default(),
                            });
                    update
                        .changed_properties
                        .insert(*prop_name, Some(Variant::Ref(*target_id)));
                }
            }
        }

        msgpack_ok(RefPatchResponse {
            session_id: self.serve_session.session_id(),
            patch: SubscribeMessage {
                added: HashMap::new(),
                removed: Vec::new(),
                updated: instance_updates.into_values().collect(),
            },
        })
    }

    /// Open a script with the given ID in the user's default text editor.
    async fn handle_api_open(&self, request: Request<Body>) -> Response<Body> {
        // Opening a file launches a local program, so it must never be reachable
        // by a remote client even when the server is bound to an exposed address.
        //
        // `remote_addr` is the immediate peer, which is the best locality signal
        // we have: the legitimate caller is a sandboxed Roblox plugin whose only
        // credential is being able to reach the port, so there is no secret to
        // authenticate it with. A connection forwarded over loopback by an
        // SSH/Tailscale tunnel or a local reverse proxy therefore appears local
        // and is allowed. That is delegated trust rather than a bypass: by
        // standing up that tunnel or proxy the user has decided the remote end is
        // trusted, and reachability is bounded by that hop's own authentication
        // (e.g. SSH keys or Tailscale ACLs). This gate only stops direct,
        // unauthenticated peers.
        //
        // An IPv4 client reaching a dual-stack (`::`) bind appears as an
        // IPv4-mapped IPv6 peer (`::ffff:127.0.0.1`), so canonicalize to the bare
        // IPv4 form before the loopback test, matching `origin`'s handling.
        if let Some(response) = self.reject_non_local("/api/open") {
            return response;
        }

        let argument = &request.uri().path()["/api/open/".len()..];
        let requested_id = match Ref::from_str(argument) {
            Ok(id) => id,
            Err(_) => {
                return msgpack(
                    ErrorResponse::bad_request("Invalid instance ID"),
                    StatusCode::BAD_REQUEST,
                );
            }
        };

        let tree = self.serve_session.tree();

        let instance = match tree.get_instance(requested_id) {
            Some(instance) => instance,
            None => {
                return msgpack(
                    ErrorResponse::bad_request("Instance not found"),
                    StatusCode::NOT_FOUND,
                );
            }
        };

        let script_path = match pick_script_path(instance) {
            Some(path) => path,
            None => {
                return msgpack(
                    ErrorResponse::bad_request(
                        "No appropriate file could be found to open this script",
                    ),
                    StatusCode::NOT_FOUND,
                );
            }
        };

        match opener::open(&script_path) {
            Ok(()) => {}
            Err(error) => match error {
                OpenError::Io(io_error) => {
                    return msgpack(
                        ErrorResponse::internal_error(format!(
                            "Attempting to open {} failed because of the following io error: {}",
                            script_path.display(),
                            io_error
                        )),
                        StatusCode::INTERNAL_SERVER_ERROR,
                    )
                }
                OpenError::ExitStatus {
                    cmd,
                    status,
                    stderr,
                } => {
                    return msgpack(
                        ErrorResponse::internal_error(format!(
                            r#"The command '{}' to open '{}' failed with the error code '{}'.
                            Error logs:
                            {}"#,
                            cmd,
                            script_path.display(),
                            status,
                            stderr
                        )),
                        StatusCode::INTERNAL_SERVER_ERROR,
                    )
                }
            },
        };

        msgpack_ok(OpenResponse {
            session_id: self.serve_session.session_id(),
        })
    }
}

fn parse_exec_session_query(
    query: Option<&str>,
) -> Result<Option<(PluginSessionId, StudioMode)>, Box<Response<Body>>> {
    let Some(query) = query else {
        return Ok(None);
    };

    let mut plugin_session_id = None;
    let mut studio_mode = None;
    for pair in query.split('&') {
        let Some((key, value)) = pair.split_once('=') else {
            return Err(Box::new(msgpack(
                ErrorResponse::bad_request("Malformed exec claim query"),
                StatusCode::BAD_REQUEST,
            )));
        };
        match key {
            "pluginSessionId" if plugin_session_id.is_none() => {
                plugin_session_id = Some(value);
            }
            "studioMode" if studio_mode.is_none() => studio_mode = Some(value),
            _ => {
                return Err(Box::new(msgpack(
                    ErrorResponse::bad_request("Malformed exec claim query"),
                    StatusCode::BAD_REQUEST,
                )));
            }
        }
    }

    parse_exec_session(plugin_session_id, studio_mode)
}

fn parse_exec_completion_session(
    plugin_session_id: Option<&str>,
    studio_mode: Option<StudioMode>,
) -> Result<Option<(PluginSessionId, StudioMode)>, Box<Response<Body>>> {
    match (plugin_session_id, studio_mode) {
        (None, None) => Ok(None),
        (Some(plugin_session_id), Some(studio_mode)) => {
            let plugin_session_id = plugin_session_id.parse().map_err(|err| {
                Box::new(msgpack(
                    ErrorResponse::bad_request(format!("Malformed plugin session ID: {err}")),
                    StatusCode::BAD_REQUEST,
                ))
            })?;
            Ok(Some((plugin_session_id, studio_mode)))
        }
        _ => Err(Box::new(msgpack(
            ErrorResponse::bad_request(
                "Exec session metadata requires both pluginSessionId and studioMode",
            ),
            StatusCode::BAD_REQUEST,
        ))),
    }
}

fn parse_exec_session(
    plugin_session_id: Option<&str>,
    studio_mode: Option<&str>,
) -> Result<Option<(PluginSessionId, StudioMode)>, Box<Response<Body>>> {
    match (plugin_session_id, studio_mode) {
        (None, None) => Ok(None),
        (Some(plugin_session_id), Some(studio_mode)) => {
            let plugin_session_id = plugin_session_id.parse().map_err(|err| {
                Box::new(msgpack(
                    ErrorResponse::bad_request(format!("Malformed plugin session ID: {err}")),
                    StatusCode::BAD_REQUEST,
                ))
            })?;
            let studio_mode = match studio_mode {
                "edit" => StudioMode::Edit,
                "play" => StudioMode::Play,
                "run" => StudioMode::Run,
                "unknown" => StudioMode::Unknown,
                _ => {
                    return Err(Box::new(msgpack(
                        ErrorResponse::bad_request("Malformed Studio mode"),
                        StatusCode::BAD_REQUEST,
                    )));
                }
            };
            Ok(Some((plugin_session_id, studio_mode)))
        }
        _ => Err(Box::new(msgpack(
            ErrorResponse::bad_request(
                "Exec claim requires both pluginSessionId and studioMode query parameters",
            ),
            StatusCode::BAD_REQUEST,
        ))),
    }
}

async fn read_bounded_body(
    mut body: Body,
    limit: usize,
    request_name: &str,
) -> Result<Vec<u8>, Response<Body>> {
    if body
        .size_hint()
        .upper()
        .is_some_and(|size| size > limit as u64)
    {
        return Err(body_too_large(request_name, limit));
    }

    let capacity = body.size_hint().lower().min(limit as u64) as usize;
    let mut bytes = Vec::with_capacity(capacity);

    while let Some(chunk) = body.data().await {
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(err) => {
                return Err(msgpack(
                    ErrorResponse::bad_request(format!("Failed to read request body: {err}")),
                    StatusCode::BAD_REQUEST,
                ));
            }
        };

        if bytes.len().saturating_add(chunk.len()) > limit {
            return Err(body_too_large(request_name, limit));
        }

        bytes.extend_from_slice(&chunk);
    }

    Ok(bytes)
}

fn body_too_large(request_name: &str, limit: usize) -> Response<Body> {
    msgpack(
        ErrorResponse::payload_too_large(format!(
            "{request_name} request body exceeds the {limit}-byte limit"
        )),
        StatusCode::PAYLOAD_TOO_LARGE,
    )
}

fn exec_store_error(error: ExecJobStoreError) -> Response<Body> {
    let details = error.to_string();

    match error {
        ExecJobStoreError::SourceTooLarge { .. } => msgpack(
            ErrorResponse::payload_too_large(details),
            StatusCode::PAYLOAD_TOO_LARGE,
        ),
        ExecJobStoreError::PendingQueueFull { .. } => msgpack(
            ErrorResponse::too_many_requests(details),
            StatusCode::TOO_MANY_REQUESTS,
        ),
        ExecJobStoreError::UnknownJob { .. } => {
            msgpack(ErrorResponse::not_found(details), StatusCode::NOT_FOUND)
        }
        ExecJobStoreError::JobNotClaimed { .. } | ExecJobStoreError::DuplicateCompletion { .. } => {
            msgpack(ErrorResponse::conflict(details), StatusCode::CONFLICT)
        }
    }
}

fn automation_store_error(error: AutomationJobStoreError) -> Response<Body> {
    let details = error.to_string();
    match error {
        AutomationJobStoreError::InvalidRequest(_) => {
            msgpack(ErrorResponse::bad_request(details), StatusCode::BAD_REQUEST)
        }
        AutomationJobStoreError::PendingQueueFull { .. } => msgpack(
            ErrorResponse::too_many_requests(details),
            StatusCode::TOO_MANY_REQUESTS,
        ),
        AutomationJobStoreError::UnknownJob { .. } => {
            msgpack(ErrorResponse::not_found(details), StatusCode::NOT_FOUND)
        }
        AutomationJobStoreError::JobNotClaimed { .. }
        | AutomationJobStoreError::DuplicateCompletion { .. }
        | AutomationJobStoreError::WrongClaimant { .. }
        | AutomationJobStoreError::ResultKindMismatch { .. } => {
            msgpack(ErrorResponse::conflict(details), StatusCode::CONFLICT)
        }
    }
}

fn into_stored_logs(logs: Vec<ExecLog>) -> Option<Vec<crate::exec::ExecLog>> {
    if logs.is_empty() {
        None
    } else {
        Some(logs.into_iter().map(Into::into).collect())
    }
}

fn no_store(mut response: Response<Body>) -> Response<Body> {
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

/// If this instance is represented by a script, try to find the correct .lua or .luau
/// file to open to edit it.
fn pick_script_path(instance: InstanceWithMeta<'_>) -> Option<PathBuf> {
    match instance.class_name().as_str() {
        "Script" | "LocalScript" | "ModuleScript" => {}
        _ => return None,
    }

    // Pick the first listed relevant path that has an extension of .lua or .luau that
    // exists.
    instance
        .metadata()
        .relevant_paths
        .iter()
        .find(|path| {
            // We should only ever open Lua or Luau files to be safe.
            match path.extension().and_then(|ext| ext.to_str()) {
                Some("lua") => {}
                Some("luau") => {}
                _ => return false,
            }

            fs::metadata(path)
                .map(|meta| meta.is_file())
                .unwrap_or(false)
        })
        .map(|path| path.to_owned())
}

/// Handle WebSocket connection for streaming subscription messages
async fn handle_websocket_subscription(
    serve_session: Arc<ServeSession>,
    websocket: HyperWebsocket,
    input_cursor: u32,
) -> anyhow::Result<()> {
    let mut websocket = websocket.await?;

    let session_id = serve_session.session_id();
    let tree_handle = serve_session.tree_handle();
    let message_queue = serve_session.message_queue();

    log::debug!(
        "WebSocket subscription established for session {}",
        session_id
    );

    // Now continuously listen for new messages using select to handle both incoming messages
    // and WebSocket control messages concurrently
    let mut cursor = input_cursor;
    loop {
        let receiver = message_queue.subscribe(cursor);

        tokio::select! {
            // Handle new messages from the message queue
            result = receiver => {
                match result {
                    Ok((new_cursor, messages)) => {
                        if !messages.is_empty() {
                            let msgpack_message = {
                                let tree = tree_handle.lock().unwrap();
                                let api_messages = messages
                                    .into_iter()
                                    .map(|patch| SubscribeMessage::from_patch_update(&tree, patch))
                                    .collect();

                                let response = SocketPacket {
                                    session_id,
                                    packet_type: SocketPacketType::Messages,
                                    body: SocketPacketBody::Messages(MessagesPacket {
                                        message_cursor: new_cursor,
                                        messages: api_messages,
                                    }),
                                };

                                serialize_msgpack(response)?
                            };

                            log::debug!("Sending batch of messages over WebSocket subscription");

                            if websocket.send(Message::Binary(msgpack_message)).await.is_err() {
                                // Client disconnected
                                log::debug!("WebSocket subscription closed by client");
                                break;
                            }
                            cursor = new_cursor;
                        }
                    }
                    Err(_) => {
                        // Message queue disconnected
                        log::debug!("Message queue disconnected; closing WebSocket subscription");
                        let _ = websocket.send(Message::Close(None)).await;
                        break;
                    }
                }
            }

            // Handle incoming WebSocket messages (ping/pong/close)
            msg = websocket.next() => {
                match msg {
                    Some(Ok(Message::Close(_))) => {
                        log::debug!("WebSocket subscription closed by client");
                        break;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        // tungstenite handles pong automatically
                        log::debug!("Received ping: {:?}", data);
                    }
                    Some(Ok(Message::Pong(data))) => {
                        log::debug!("Received pong: {:?}", data);
                    }
                    Some(Ok(Message::Text(_))) | Some(Ok(Message::Binary(_))) => {
                        // Ignore text/binary messages from client for subscription endpoint
                        // TODO: Use this for bidirectional sync or requesting fallbacks?
                        log::debug!("Ignoring message from client since we don't use it for anything yet: {:?}", msg);
                    }
                    Some(Ok(Message::Frame(_))) => {
                        // This should never happen according to tungstenite docs
                        unreachable!();
                    }
                    Some(Err(e)) => {
                        log::error!("WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        // WebSocket stream ended
                        log::debug!("WebSocket stream ended");
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

/// Certain Instances MUST be a child of specific classes. This function
/// tracks that information for the Serialize endpoint.
///
/// If a parent requirement exists, it will be returned.
/// Otherwise returns `None`.
fn parent_requirements(class: &str) -> Option<&str> {
    Some(match class {
        "Attachment" | "Bone" => "Part",
        "Animator" => "Humanoid",
        "BaseWrap" | "WrapLayer" | "WrapTarget" | "WrapDeformer" => "MeshPart",
        _ => return None,
    })
}

#[cfg(test)]
mod test {
    use memofs::{StdBackend, Vfs};

    use super::*;

    #[tokio::test]
    async fn local_automation_routes_reject_non_loopback_peers() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("default.project.json"),
            r#"{ "name": "test", "tree": { "$className": "Folder" } }"#,
        )
        .unwrap();

        let start_path = std::fs::canonicalize(dir.path()).unwrap();
        let vfs = Vfs::new(StdBackend::new().unwrap());
        let serve_session = Arc::new(ServeSession::new(vfs, start_path).unwrap());
        let id = Uuid::new_v4();
        let routes = [
            (Method::POST, "/api/exec/jobs".to_owned()),
            (Method::GET, "/api/exec/jobs/next".to_owned()),
            (Method::GET, format!("/api/exec/jobs/{id}")),
            (Method::POST, format!("/api/exec/jobs/{id}/complete")),
            (Method::POST, "/api/automation/jobs".to_owned()),
            (Method::GET, "/api/automation/jobs/next".to_owned()),
            (Method::GET, format!("/api/automation/jobs/{id}")),
            (Method::POST, format!("/api/automation/jobs/{id}/complete")),
            (Method::GET, "/api/automation/status".to_owned()),
            (Method::POST, "/api/automation/status".to_owned()),
        ];

        for (method, route) in routes {
            let request = Request::builder()
                .method(method)
                .uri(route)
                .body(Body::empty())
                .unwrap();
            let response = call(
                Arc::clone(&serve_session),
                "192.0.2.1:4567".parse().unwrap(),
                request,
            )
            .await;

            assert_eq!(response.status(), StatusCode::FORBIDDEN);
            let body = body::to_bytes(response.into_body()).await.unwrap();
            let error: serde_json::Value = deserialize_msgpack(&body).unwrap();
            assert_eq!(error["kind"], "Forbidden");
        }
    }
}
