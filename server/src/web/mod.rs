mod api;
mod interface;

use std::sync::Arc;

use log::trace;
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

use crate::serve_session::ServeSession;

use self::{
    api::ApiService,
    interface::InterfaceService,
};

pub struct RootService {
    api: api::ApiService,
    interface: interface::InterfaceService,
}

impl Service for RootService {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = hyper::Error;
    type Future = Box<dyn Future<Item = Response<Self::ReqBody>, Error = Self::Error> + Send>;

    fn call(&mut self, request: Request<Self::ReqBody>) -> Self::Future {
        trace!("{} {}", request.method(), request.uri().path());

        if request.uri().path().starts_with("/api") {
            self.api.call(request)
        } else {
            self.interface.call(request)
        }
    }
}

impl RootService {
    pub fn new(serve_session: Arc<ServeSession>) -> RootService {
        RootService {
            api: ApiService::new(Arc::clone(&serve_session)),
            interface: InterfaceService::new(Arc::clone(&serve_session)),
        }
    }
}

pub struct LiveServer {
    serve_session: Arc<ServeSession>,
}

impl LiveServer {
    pub fn new(serve_session: Arc<ServeSession>) -> LiveServer {
        LiveServer {
            serve_session,
        }
    }

    pub fn start(self, port: u16) {
        let address = ([127, 0, 0, 1], port).into();

        let server = Server::bind(&address)
            .serve(move || {
                let service: FutureResult<_, hyper::Error> =
                    future::ok(RootService::new(Arc::clone(&self.serve_session)));
                service
            })
            .map_err(|e| eprintln!("Server error: {}", e));

        hyper::rt::run(server);
    }
}