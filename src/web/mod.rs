//! Defines the Rojo web interface. This is what the Roblox Studio plugin
//! communicates with. Eventually, we'll make this API stable, produce better
//! documentation for it, and open it up for other consumers.

mod api;
mod assets;
pub mod interface;
mod ui;
mod util;

use std::{net::SocketAddr, sync::Arc};

use futures::{
    future::{self, FutureResult},
    Future,
};
use hyper::{service::Service, Body, Request, Response, Server};
use log::trace;

use crate::serve_session::ServeSession;

use self::{api::ApiService, ui::UiService};

pub struct RootService {
    api: ApiService,
    ui: UiService,
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
            self.ui.call(request)
        }
    }
}

impl RootService {
    pub fn new(serve_session: Arc<ServeSession>) -> Self {
        RootService {
            api: ApiService::new(Arc::clone(&serve_session)),
            ui: UiService::new(Arc::clone(&serve_session)),
        }
    }
}

pub struct LiveServer {
    serve_session: Arc<ServeSession>,
}

impl LiveServer {
    pub fn new(serve_session: Arc<ServeSession>) -> Self {
        LiveServer { serve_session }
    }

    pub fn start(self, address: SocketAddr) {
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
