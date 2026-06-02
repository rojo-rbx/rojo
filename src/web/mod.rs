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

use anyhow::Context;
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

    /// Starts the server on the given address, blocking until it stops.
    ///
    /// `on_listening` is invoked once the server has successfully bound to the
    /// address, so callers can defer printing any "listening" message until
    /// after binding can no longer fail (e.g. due to the port being in use).
    pub fn start(self, address: SocketAddr, on_listening: impl FnOnce()) -> anyhow::Result<()> {
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

        let rt = Runtime::new().context("Failed to start the async runtime for the web server")?;
        let _guard = rt.enter();
        let server = Server::try_bind(&address)
            .with_context(|| {
                format!(
                    "Could not start the Rojo server on {address}.\n\
                     The address may already be in use or reserved. Another Rojo server might already \
                     be running, or another program may be using that port.\n\
                     You can pick a different port with the --port option."
                )
            })?
            .serve(make_service);

        // Binding succeeded, so it's now safe to tell the user we're listening.
        on_listening();

        rt.block_on(server)
            .context("The Rojo web server encountered a fatal error")?;

        Ok(())
    }
}
