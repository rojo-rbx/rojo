//! Defines Rojo's HTTP API, all under /api. These endpoints generally return
//! JSON.

use std::{collections::HashMap, sync::Arc};

use futures::{future, Future};

use hyper::{header, service::Service, Body, Method, Request, Response, StatusCode};
use rbx_dom_weak::RbxId;

use crate::{
    serve_session::ServeSession,
    web::{
        interface::{
            Instance, NotFoundError, ReadResponse, ServerInfoResponse, SubscribeResponse,
            PROTOCOL_VERSION, SERVER_VERSION,
        },
        util::{json, json_ok},
    },
};

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
        match (request.method(), request.uri().path()) {
            (&Method::GET, "/api/rojo") => self.handle_api_rojo(),
            (&Method::GET, path) if path.starts_with("/api/read/") => self.handle_api_read(request),
            (&Method::GET, path) if path.starts_with("/api/subscribe/") => {
                self.handle_api_subscribe(request)
            }
            _ => json(NotFoundError, StatusCode::NOT_FOUND),
        }
    }
}

impl ApiService {
    pub fn new(serve_session: Arc<ServeSession>) -> ApiService {
        ApiService { serve_session }
    }

    /// Get a summary of information about the server
    fn handle_api_rojo(&self) -> <ApiService as Service>::Future {
        let tree = self.serve_session.tree();
        let root_instance_id = tree.get_root_id();

        json_ok(&ServerInfoResponse {
            server_version: SERVER_VERSION.to_owned(),
            protocol_version: PROTOCOL_VERSION,
            session_id: self.serve_session.session_id(),
            expected_place_ids: self.serve_session.serve_place_ids().cloned(),
            root_instance_id,
        })
    }

    /// Retrieve any messages past the given cursor index, and if
    /// there weren't any, subscribe to receive any new messages.
    fn handle_api_subscribe(&self, request: Request<Body>) -> <ApiService as Service>::Future {
        let argument = &request.uri().path()["/api/subscribe/".len()..];
        let _input_cursor: u32 = match argument.parse() {
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

        let message_queue = self.serve_session.message_queue();
        let message_cursor = message_queue.cursor();

        let messages = Vec::new(); // TODO

        json_ok(SubscribeResponse {
            session_id: self.serve_session.session_id(),
            message_cursor,
            messages,
        })
    }

    fn handle_api_read(&self, request: Request<Body>) -> <ApiService as Service>::Future {
        let argument = &request.uri().path()["/api/read/".len()..];
        let requested_ids: Option<Vec<RbxId>> = argument.split(',').map(RbxId::parse_str).collect();

        let requested_ids = match requested_ids {
            Some(ids) => ids,
            None => {
                return Box::new(future::ok(
                    Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .header(header::CONTENT_TYPE, "text/plain")
                        .body(Body::from("Malformed ID list"))
                        .unwrap(),
                ));
            }
        };

        let message_queue = self.serve_session.message_queue();
        let message_cursor = message_queue.cursor();

        let tree = self.serve_session.tree();
        let inner_tree = tree.inner();

        let mut instances = HashMap::new();

        for id in requested_ids {
            if let Some(instance) = inner_tree.get_instance(id) {
                instances.insert(id, Instance::from_rbx_instance(instance));

                for descendant in inner_tree.descendants(id) {
                    instances.insert(descendant.get_id(), Instance::from_rbx_instance(descendant));
                }
            }
        }

        json_ok(ReadResponse {
            session_id: self.serve_session.session_id(),
            message_cursor,
            instances,
        })
    }
}
