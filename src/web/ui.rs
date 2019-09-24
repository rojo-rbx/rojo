//! Defines the HTTP-based UI. These endpoints generally return HTML and SVG.

use std::{sync::Arc, time::Duration};

use futures::{future, Future};
use hyper::{header, service::Service, Body, Method, Request, Response, StatusCode};
use ritz::html;

use crate::{
    imfs::ImfsFetcher,
    serve_session::ServeSession,
    web::{
        interface::{ErrorResponse, SERVER_VERSION},
        util::json,
    },
};

static LOGO: &[u8] = include_bytes!("../../assets/logo-512.png");
static ICON: &[u8] = include_bytes!("../../assets/icon-32.png");
static HOME_CSS: &str = include_str!("../../assets/index.css");

pub struct UiService<F> {
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
            (&Method::GET, "/logo.png") => self.handle_logo(),
            (&Method::GET, "/icon.png") => self.handle_icon(),
            (&Method::GET, "/visualize/rbx") => self.handle_visualize_rbx(),
            (&Method::GET, "/visualize/imfs") => self.handle_visualize_imfs(),
            (_method, path) => {
                return json(
                    ErrorResponse::not_found(format!("Route not found: {}", path)),
                    StatusCode::NOT_FOUND,
                )
            }
        };

        Box::new(future::ok(response))
    }
}

impl<F: ImfsFetcher> UiService<F> {
    pub fn new(serve_session: Arc<ServeSession<F>>) -> Self {
        UiService { serve_session }
    }

    fn handle_logo(&self) -> Response<Body> {
        Response::builder()
            .header(header::CONTENT_TYPE, "image/png")
            .body(Body::from(LOGO))
            .unwrap()
    }

    fn handle_icon(&self) -> Response<Body> {
        Response::builder()
            .header(header::CONTENT_TYPE, "image/png")
            .body(Body::from(ICON))
            .unwrap()
    }

    fn handle_home(&self) -> Response<Body> {
        let project_name = self.serve_session.project_name().unwrap_or("<unnamed>");
        let uptime = {
            let elapsed = self.serve_session.start_time().elapsed();

            // Round off all of our sub-second precision to make timestamps
            // nicer.
            let just_nanos = Duration::from_nanos(elapsed.subsec_nanos() as u64);
            let elapsed = elapsed - just_nanos;

            humantime::format_duration(elapsed).to_string()
        };

        let page = html! {
            <html>
                <head>
                    <title>"Rojo Live Server"</title>
                    <link rel="icon" type="image/png" sizes="32x32" href="/icon.png" />
                    <style>
                        { ritz::UnescapedText::new(HOME_CSS) }
                    </style>
                </head>

                <body>
                    <main class="main">
                        <header class="header">
                            <img class="main-logo" src="/logo.png" />
                            <div class="stats">
                                <span class="stat">
                                    <span class="stat-name">"Server Version: "</span>
                                    <span class="stat-value">{ SERVER_VERSION }</span>
                                </span>
                                <span class="stat">
                                    <span class="stat-name">"Project: "</span>
                                    <span class="stat-value">{ project_name }</span>
                                </span>
                                <span class="stat">
                                    <span class="stat-name">"Server Uptime: "</span>
                                    <span class="stat-value">{ uptime.to_string() }</span>
                                </span>
                            </div>
                        </header>
                        <div class="button-list">
                            <a class="button" href="https://rojo.space/docs">
                                "Rojo Documentation"
                            </a>
                            <a class="button" href="/visualize/imfs">
                                "View in-memory filesystem state"
                            </a>
                            <a class="button" href="/visualize/rbx">
                                "View instance tree state"
                            </a>
                        </div>
                    </main>
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
