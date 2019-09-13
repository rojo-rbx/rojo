//! Defines the HTTP-based UI. These endpoints generally return HTML and SVG.

use std::sync::Arc;

use futures::{future, Future};
use hyper::{header, service::Service, Body, Method, Request, Response, StatusCode};
use ritz::html;

use crate::{
    imfs::new::ImfsFetcher,
    serve_session::ServeSession,
    web::{
        interface::{NotFoundError, SERVER_VERSION},
        util::json,
    },
};

static HOME_CSS: &str = include_str!("../../assets/index.css");

pub struct UiService<F> {
    #[allow(unused)] // TODO: Fill out interface service
    serve_session: Arc<ServeSession<F>>,
}

impl<F: ImfsFetcher> Service for UiService<F> {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = hyper::Error;
    type Future = Box<dyn Future<Item = Response<Self::ReqBody>, Error = Self::Error> + Send>;

    fn call(&mut self, request: Request<Self::ReqBody>) -> Self::Future {
        let response = match (request.method(), request.uri().path()) {
            (&Method::GET, "/") => self.handle_home(),
            (&Method::GET, "/visualize/rbx") => self.handle_visualize_rbx(),
            (&Method::GET, "/visualize/imfs") => self.handle_visualize_imfs(),
            _ => return json(NotFoundError, StatusCode::NOT_FOUND),
        };

        Box::new(future::ok(response))
    }
}

impl<F: ImfsFetcher> UiService<F> {
    pub fn new(serve_session: Arc<ServeSession<F>>) -> Self {
        UiService { serve_session }
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
                            "Version " { SERVER_VERSION }
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
        Response::builder()
            .header(header::CONTENT_TYPE, "text/plain")
            .body(Body::from("TODO: /visualize/rbx"))
            .unwrap()
    }

    fn handle_visualize_imfs(&self) -> Response<Body> {
        Response::builder()
            .header(header::CONTENT_TYPE, "text/plain")
            .body(Body::from("TODO: /visualize/imfs"))
            .unwrap()
    }
}
