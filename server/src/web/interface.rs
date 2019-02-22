//! Defines the HTTP-based UI. These endpoints generally return HTML and SVG.

use std::sync::Arc;

use futures::{future, Future};
use hyper::{
    service::Service,
    header,
    Body,
    Method,
    StatusCode,
    Request,
    Response,
};
use ritz::html;

use crate::{
    live_session::LiveSession,
    visualize::{VisualizeRbxSession, VisualizeImfs, graphviz_to_svg},
};

static HOME_CSS: &str = include_str!("../../assets/index.css");

pub struct InterfaceService {
    live_session: Arc<LiveSession>,
    server_version: &'static str,
}

impl Service for InterfaceService {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = hyper::Error;
    type Future = Box<dyn Future<Item = Response<Self::ReqBody>, Error = Self::Error> + Send>;

    fn call(&mut self, request: Request<Self::ReqBody>) -> Self::Future {
        let response = match (request.method(), request.uri().path()) {
            (&Method::GET, "/") => self.handle_home(),
            (&Method::GET, "/visualize/rbx") => self.handle_visualize_rbx(),
            (&Method::GET, "/visualize/imfs") => self.handle_visualize_imfs(),
            _ => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())
                .unwrap(),
        };

        Box::new(future::ok(response))
    }
}

impl InterfaceService {
    pub fn new(live_session: Arc<LiveSession>) -> InterfaceService {
        InterfaceService {
            live_session,
            server_version: env!("CARGO_PKG_VERSION"),
        }
    }

    fn handle_home(&self) -> Response<Body> {
        let page = html! {
            <html>
                <head>
                    <title>"Rojo"</title>
                    <style>
                        { ritz::UnescapedText::new(HOME_CSS) }
                    </style>
                </head>

                <body>
                    <div class="main">
                        <h1 class="title">
                            "Rojo Live Sync is up and running!"
                        </h1>
                        <h2 class="subtitle">
                            "Version " { self.server_version }
                        </h2>
                        <a class="docs" href="https://lpghatguy.github.io/rojo">
                            "Rojo Documentation"
                        </a>
                    </div>
                </body>
            </html>
        };

        Response::builder()
            .header(header::CONTENT_TYPE, "text/html")
            .body(Body::from(format!("<!DOCTYPE html>{}", page)))
            .unwrap()
    }

    fn handle_visualize_rbx(&self) -> Response<Body> {
        let rbx_session = self.live_session.rbx_session.lock().unwrap();
        let dot_source = format!("{}", VisualizeRbxSession(&rbx_session));

        match graphviz_to_svg(&dot_source) {
            Some(svg) => Response::builder()
                .header(header::CONTENT_TYPE, "image/svg+xml")
                .body(Body::from(svg))
                .unwrap(),
            None => Response::builder()
                .header(header::CONTENT_TYPE, "text/plain")
                .body(Body::from(dot_source))
                .unwrap(),
        }
    }

    fn handle_visualize_imfs(&self) -> Response<Body> {
        let imfs = self.live_session.imfs.lock().unwrap();
        let dot_source = format!("{}", VisualizeImfs(&imfs));

        match graphviz_to_svg(&dot_source) {
            Some(svg) => Response::builder()
                .header(header::CONTENT_TYPE, "image/svg+xml")
                .body(Body::from(svg))
                .unwrap(),
            None => Response::builder()
                .header(header::CONTENT_TYPE, "text/plain")
                .body(Body::from(dot_source))
                .unwrap(),
        }
    }
}