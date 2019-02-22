// TODO: This module needs to be public for visualize, we should move
// PublicInstanceMetadata and switch this private!
pub mod api;
mod interface;

use std::sync::Arc;

use futures::{
    future::{self, FutureResult},
    Future,
};
use hyper::{
    service::Service,
    Body,
    Request,
    Response,
    Server,
};

use crate::{
    live_session::LiveSession,
};

use self::{
    api::ApiServer,
    interface::InterfaceServer,
};

pub struct RootService {
    api: api::ApiServer,
    interface: interface::InterfaceServer,
}

impl Service for RootService {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = hyper::Error;
    type Future = Box<Future<Item = Response<Self::ReqBody>, Error = Self::Error> + Send>;

    fn call(&mut self, request: Request<Self::ReqBody>) -> Self::Future {
        if request.uri().path().starts_with("/api") {
            self.api.call(request)
        } else {
            self.interface.call(request)
        }
    }
}

impl RootService {
    pub fn new(live_session: Arc<LiveSession>) -> RootService {
        RootService {
            api: ApiServer::new(Arc::clone(&live_session)),
            interface: InterfaceServer::new(Arc::clone(&live_session)),
        }
    }
}

pub struct LiveServer {
    live_session: Arc<LiveSession>,
}

impl LiveServer {
    pub fn new(live_session: Arc<LiveSession>) -> LiveServer {
        LiveServer {
            live_session,
        }
    }

    pub fn start(self, port: u16) {
        let address = ([127, 0, 0, 1], port).into();

        let server = Server::bind(&address)
            .serve(move || {
                let service: FutureResult<RootService, hyper::Error> =
                    future::ok(RootService::new(Arc::clone(&self.live_session)));
                service
            })
            .map_err(|e| eprintln!("Server error: {}", e));

        hyper::rt::run(server);
    }
}