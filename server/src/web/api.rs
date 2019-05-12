//! Defines Rojo's HTTP API, all under /api. These endpoints generally return
//! JSON.

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    sync::Arc,
};

use futures::{
    future::{self, IntoFuture},
    Future,
    sync::oneshot,
};
use hyper::{
    service::Service,
    header,
    StatusCode,
    Method,
    Body,
    Request,
    Response,
};
use serde::{Serialize, Deserialize};
use rbx_dom_weak::{RbxId, RbxInstance};

use crate::{
    live_session::LiveSession,
    session_id::SessionId,
    snapshot_reconciler::InstanceChanges,
    rbx_session::{MetadataPerInstance},
};

/// Contains the instance metadata relevant to Rojo clients.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicInstanceMetadata {
    ignore_unknown_instances: bool,
}

impl PublicInstanceMetadata {
    pub fn from_session_metadata(meta: &MetadataPerInstance) -> PublicInstanceMetadata {
        PublicInstanceMetadata {
            ignore_unknown_instances: meta.ignore_unknown_instances,
        }
    }
}

/// Used to attach metadata specific to Rojo to instances, which come from the
/// rbx_dom_weak crate.
///
/// Both fields are wrapped in Cow in order to make owned-vs-borrowed simpler
/// for tests.
#[derive(Debug, Serialize, Deserialize)]
pub struct InstanceWithMetadata<'a> {
    #[serde(flatten)]
    pub instance: Cow<'a, RbxInstance>,

    #[serde(rename = "Metadata")]
    pub metadata: Option<PublicInstanceMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfoResponse<'a> {
    pub session_id: SessionId,
    pub server_version: &'a str,
    pub protocol_version: u64,
    pub expected_place_ids: Option<HashSet<u64>>,
    pub root_instance_id: RbxId,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadResponse<'a> {
    pub session_id: SessionId,
    pub message_cursor: u32,
    pub instances: HashMap<RbxId, InstanceWithMetadata<'a>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeResponse<'a> {
    pub session_id: SessionId,
    pub message_cursor: u32,
    pub messages: Cow<'a, [InstanceChanges]>,
}

fn response_json<T: serde::Serialize>(value: T) -> Response<Body> {
    let serialized = match serde_json::to_string(&value) {
        Ok(v) => v,
        Err(err) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::CONTENT_TYPE, "text/plain")
                .body(Body::from(err.to_string()))
                .unwrap();
        },
    };

    Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serialized))
        .unwrap()
}

pub struct ApiService {
    live_session: Arc<LiveSession>,
    server_version: &'static str,
}

impl Service for ApiService {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = hyper::Error;
    type Future = Box<dyn Future<Item = hyper::Response<Self::ReqBody>, Error = Self::Error> + Send>;

    fn call(&mut self, request: hyper::Request<Self::ReqBody>) -> Self::Future {
        let response = match (request.method(), request.uri().path()) {
            (&Method::GET, "/api/rojo") => self.handle_api_rojo(),
            (&Method::GET, path) if path.starts_with("/api/read/") => self.handle_api_read(request),
            (&Method::GET, path) if path.starts_with("/api/subscribe/") => {
                return self.handle_api_subscribe(request);
            }
            _ => {
                Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::empty())
                    .unwrap()
            }
        };

        Box::new(future::ok(response))
    }
}

impl ApiService {
    pub fn new(live_session: Arc<LiveSession>) -> ApiService {
        ApiService {
            live_session,
            server_version: env!("CARGO_PKG_VERSION"),
        }
    }

    /// Get a summary of information about the server
    fn handle_api_rojo(&self) -> Response<Body> {
        let rbx_session = self.live_session.rbx_session.lock().unwrap();
        let tree = rbx_session.get_tree();

        response_json(&ServerInfoResponse {
            server_version: self.server_version,
            protocol_version: 2,
            session_id: self.live_session.session_id(),
            expected_place_ids: self.live_session.serve_place_ids().clone(),
            root_instance_id: tree.get_root_id(),
        })
    }

    /// Retrieve any messages past the given cursor index, and if
    /// there weren't any, subscribe to receive any new messages.
    fn handle_api_subscribe(&self, request: Request<Body>) -> <ApiService as Service>::Future {
        let argument = &request.uri().path()["/api/subscribe/".len()..];
        let cursor: u32 = match argument.parse() {
            Ok(v) => v,
            Err(err) => {
                return Box::new(future::ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header(header::CONTENT_TYPE, "text/plain")
                    .body(Body::from(err.to_string()))
                    .unwrap()));
            },
        };

        let message_queue = Arc::clone(&self.live_session.message_queue);
        let session_id = self.live_session.session_id();

        let (tx, rx) = oneshot::channel();
        message_queue.subscribe(cursor, tx);

        let result = rx.into_future()
            .and_then(move |(new_cursor, new_messages)| {
                Box::new(future::ok(response_json(SubscribeResponse {
                    session_id: session_id,
                    messages: Cow::Owned(new_messages),
                    message_cursor: new_cursor,
                })))
            })
            .or_else(|e| {
                Box::new(future::ok(Response::builder()
                    .status(500)
                    .body(Body::from(format!("Internal Error: {:?}", e)))
                    .unwrap()))
            });

        Box::new(result)
    }

    fn handle_api_read(&self, request: Request<Body>) -> Response<Body> {
        let argument = &request.uri().path()["/api/read/".len()..];
        let requested_ids: Option<Vec<RbxId>> = argument
            .split(',')
            .map(RbxId::parse_str)
            .collect();

        let message_queue = Arc::clone(&self.live_session.message_queue);

        let requested_ids = match requested_ids {
            Some(id) => id,
            None => {
                return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header(header::CONTENT_TYPE, "text/plain")
                    .body(Body::from("Malformed ID list"))
                    .unwrap();
            },
        };

        let rbx_session = self.live_session.rbx_session.lock().unwrap();
        let tree = rbx_session.get_tree();

        let message_cursor = message_queue.get_message_cursor();

        let mut instances = HashMap::new();

        for &requested_id in &requested_ids {
            if let Some(instance) = tree.get_instance(requested_id) {
                let metadata = rbx_session.get_instance_metadata(requested_id)
                    .map(PublicInstanceMetadata::from_session_metadata);

                instances.insert(instance.get_id(), InstanceWithMetadata {
                    instance: Cow::Borrowed(instance),
                    metadata,
                });

                for descendant in tree.descendants(requested_id) {
                    let descendant_meta = rbx_session.get_instance_metadata(descendant.get_id())
                        .map(PublicInstanceMetadata::from_session_metadata);

                    instances.insert(descendant.get_id(), InstanceWithMetadata {
                        instance: Cow::Borrowed(descendant),
                        metadata: descendant_meta,
                    });
                }
            }
        }

        response_json(&ReadResponse {
            session_id: self.live_session.session_id(),
            message_cursor,
            instances,
        })
    }
}