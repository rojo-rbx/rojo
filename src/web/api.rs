//! Defines Rojo's HTTP API, all under /api. These endpoints generally return
//! JSON.

use std::{collections::HashSet, sync::Arc};

use futures::{future, Future};

use hyper::{header, service::Service, Body, Method, Request, Response, StatusCode};
use rbx_dom_weak::RbxId;
use serde::{Deserialize, Serialize};

use crate::{serve_session::ServeSession, session_id::SessionId};

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const PROTOCOL_VERSION: u64 = 3;

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

fn response_json<T: serde::Serialize>(value: T) -> Response<Body> {
    let serialized = match serde_json::to_string(&value) {
        Ok(v) => v,
        Err(err) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header(header::CONTENT_TYPE, "text/plain")
                .body(Body::from(err.to_string()))
                .unwrap();
        }
    };

    Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serialized))
        .unwrap()
}

pub struct ApiService {
    serve_session: Arc<ServeSession>,
}

impl Service for ApiService {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = hyper::Error;
    type Future =
        Box<dyn Future<Item = hyper::Response<Self::ReqBody>, Error = Self::Error> + Send>;

    fn call(&mut self, request: hyper::Request<Self::ReqBody>) -> Self::Future {
        let response = match (request.method(), request.uri().path()) {
            (&Method::GET, "/api/rojo") => self.handle_api_rojo(),
            (&Method::GET, path) if path.starts_with("/api/read/") => self.handle_api_read(request),
            (&Method::GET, path) if path.starts_with("/api/subscribe/") => {
                return self.handle_api_subscribe(request);
            }
            _ => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())
                .unwrap(),
        };

        Box::new(future::ok(response))
    }
}

impl ApiService {
    pub fn new(serve_session: Arc<ServeSession>) -> ApiService {
        ApiService { serve_session }
    }

    /// Get a summary of information about the server
    fn handle_api_rojo(&self) -> Response<Body> {
        response_json(&ServerInfoResponse {
            server_version: SERVER_VERSION,
            protocol_version: PROTOCOL_VERSION,
            session_id: self.serve_session.session_id(),
            expected_place_ids: self.serve_session.serve_place_ids().map(Clone::clone),
        })
    }

    /// Retrieve any messages past the given cursor index, and if
    /// there weren't any, subscribe to receive any new messages.
    fn handle_api_subscribe(&self, request: Request<Body>) -> <ApiService as Service>::Future {
        let argument = &request.uri().path()["/api/subscribe/".len()..];
        let _cursor: u32 = match argument.parse() {
            Ok(v) => v,
            Err(err) => {
                return Box::new(future::ok(
                    Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .header(header::CONTENT_TYPE, "text/plain")
                        .body(Body::from(err.to_string()))
                        .unwrap(),
                ));
            }
        };

        Box::new(future::ok(response_json(SubscribeResponse {
            session_id: self.serve_session.session_id(),
        })))
    }

    fn handle_api_read(&self, request: Request<Body>) -> Response<Body> {
        let argument = &request.uri().path()["/api/read/".len()..];
        let requested_ids: Option<Vec<RbxId>> = argument.split(',').map(RbxId::parse_str).collect();

        let _requested_ids = match requested_ids {
            Some(id) => id,
            None => {
                return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header(header::CONTENT_TYPE, "text/plain")
                    .body(Body::from("Malformed ID list"))
                    .unwrap();
            }
        };

        response_json(ReadResponse {
            session_id: self.serve_session.session_id(),
        })
    }
}
