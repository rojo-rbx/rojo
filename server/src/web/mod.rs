// TODO: This module needs to be public for visualize, we should move
// PublicInstanceMetadata and switch this private!
pub mod api;
mod interface;

use std::sync::Arc;

use log::trace;
use rouille::{find_route, Request, Response};

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

impl Server {
    pub fn new(live_session: Arc<LiveSession>) -> Server {
        Server {
            api: ApiServer::new(Arc::clone(&live_session)),
            interface: InterfaceServer::new(Arc::clone(&live_session)),
        }
    }

    pub fn handle_request(&self, request: &Request) -> Response {
        trace!("Request {} {}", request.method(), request.url());

        find_route!(
            self.api.handle_request(request),
            self.interface.handle_request(request)
        )
    }

    pub fn listen(self, port: u16) {
        let address = format!("0.0.0.0:{}", port);

        rouille::start_server(address, move |request| self.handle_request(request));
    }
}