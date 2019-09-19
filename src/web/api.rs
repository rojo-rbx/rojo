//! Defines Rojo's HTTP API, all under /api. These endpoints generally return
//! JSON.

use std::{collections::HashMap, sync::Arc};

use futures::{sync::oneshot, Future};

use hyper::{service::Service, Body, Method, Request, StatusCode};
use rbx_dom_weak::RbxId;

use crate::{
    imfs::ImfsFetcher,
    serve_session::ServeSession,
    web::{
        interface::{
            ErrorResponse, Instance, ReadResponse, ServerInfoResponse, SubscribeResponse,
            PROTOCOL_VERSION, SERVER_VERSION,
        },
        util::{json, json_ok},
    },
};

pub struct ApiService<F> {
    serve_session: Arc<ServeSession<F>>,
}

impl<F: ImfsFetcher> Service for ApiService<F> {
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
            (_method, path) => {
                return json(
                    ErrorResponse::not_found(format!("Route not found: {}", path)),
                    StatusCode::NOT_FOUND,
                )
            }
        }
    }
}

impl<F: ImfsFetcher> ApiService<F> {
    pub fn new(serve_session: Arc<ServeSession<F>>) -> Self {
        ApiService { serve_session }
    }

    /// Get a summary of information about the server
    fn handle_api_rojo(&self) -> <Self as Service>::Future {
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
    fn handle_api_subscribe(&self, request: Request<Body>) -> <Self as Service>::Future {
        let argument = &request.uri().path()["/api/subscribe/".len()..];
        let input_cursor: u32 = match argument.parse() {
            Ok(v) => v,
            Err(err) => {
                return json(
                    ErrorResponse::bad_request(format!("Malformed message cursor: {}", err)),
                    StatusCode::BAD_REQUEST,
                );
            }
        };

        let session_id = self.serve_session.session_id();
        let (sender, receiver) = oneshot::channel();

        {
            let message_queue = self.serve_session.message_queue();
            message_queue.subscribe(input_cursor, sender);
        }

        Box::new(receiver.then(move |result| match result {
            Ok((message_cursor, messages)) => json_ok(SubscribeResponse {
                session_id,
                message_cursor,
                messages,
            }),
            Err(_) => json(
                ErrorResponse::internal_error("Message queue disconnected sender"),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        }))
    }

    fn handle_api_read(&self, request: Request<Body>) -> <Self as Service>::Future {
        let argument = &request.uri().path()["/api/read/".len()..];
        let requested_ids: Option<Vec<RbxId>> = argument.split(',').map(RbxId::parse_str).collect();

        let requested_ids = match requested_ids {
            Some(ids) => ids,
            None => {
                return json(
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

        json_ok(ReadResponse {
            session_id: self.serve_session.session_id(),
            message_cursor,
            instances,
        })
    }
}
