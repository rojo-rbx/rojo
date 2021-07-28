//! Defines the Rojo web interface. This is what the Roblox Studio plugin
//! communicates with. Eventually, we'll make this API stable, produce better
//! documentation for it, and open it up for other consumers.

mod api;
mod assets;
pub mod interface;
mod ui;
mod util;

use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use hyper::{
    server::Server,
    service::{make_service_fn, service_fn},
    Body, Request,
};
use tokio::runtime::Runtime;

use crate::serve_session::ServeSession;

pub struct LiveServer {
    serve_session: Arc<ServeSession>,
}

impl LiveServer {
    pub fn new(serve_session: Arc<ServeSession>) -> Self {
        LiveServer { serve_session }
    }

    pub fn start(self, address: SocketAddr) {
        let serve_session = Arc::clone(&self.serve_session);

        let make_service = make_service_fn(move |_conn| {
            let serve_session = Arc::clone(&serve_session);

            async {
                let service = move |req: Request<Body>| {
                    let serve_session = Arc::clone(&serve_session);

                    async move {
                        if req.uri().path().starts_with("/api") {
                            Ok::<_, Infallible>(api::call(serve_session, req).await)
                        } else {
                            Ok::<_, Infallible>(ui::call(serve_session, req).await)
                        }
                    }
                };

                Ok::<_, Infallible>(service_fn(service))
            }
        });

        let rt = Runtime::new().unwrap();
        let _guard = rt.enter();
        let server = Server::bind(&address).serve(make_service);
        rt.block_on(server).unwrap();
    }
}
