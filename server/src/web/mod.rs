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
};

use crate::{
    live_session::LiveSession,
};

use self::{
    api::ApiServer,
    interface::InterfaceServer,
};

pub struct Server {
    api: api::ApiServer,
    interface: interface::InterfaceServer,
}

struct Blah;

impl Service for Blah {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = hyper::Error;
    type Future = Box<Future<Item = Response<Self::ReqBody>, Error = Self::Error> + Send>;

    fn call(&mut self, request: Request<Self::ReqBody>) -> Self::Future {
        Box::new(future::ok(Response::new(Body::from("Hello, world!"))))
    }
}

impl Server {
    pub fn new(live_session: Arc<LiveSession>) -> Server {
        Server {
            api: ApiServer::new(Arc::clone(&live_session)),
            interface: InterfaceServer::new(Arc::clone(&live_session)),
        }
    }

    pub fn listen(self, port: u16) {
        let address = ([127, 0, 0, 1], port).into();

        let server = hyper::Server::bind(&address)
            .serve(move || {
                let service: FutureResult<Blah, hyper::Error> = future::ok(Blah);
                service
            })
            .map_err(|e| eprintln!("Server error: {}", e));

        hyper::rt::run(server);
    }
}