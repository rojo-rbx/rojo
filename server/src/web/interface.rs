//! Defines the HTTP-based UI. These endpoints generally return HTML and SVG.

use std::sync::Arc;

use rouille::{
    self,
    router,
    Request,
    Response,
};
use ritz::{html};

use crate::{
    live_session::LiveSession,
    visualize::{VisualizeRbxSession, VisualizeImfs, graphviz_to_svg},
};

static HOME_CSS: &str = include_str!("../../assets/index.css");

pub struct InterfaceServer {
    live_session: Arc<LiveSession>,
    server_version: &'static str,
}

impl InterfaceServer {
    pub fn new(live_session: Arc<LiveSession>) -> InterfaceServer {
        InterfaceServer {
            live_session,
            server_version: env!("CARGO_PKG_VERSION"),
        }
    }

    #[allow(unreachable_code)]
    pub fn handle_request(&self, request: &Request) -> Response {
        router!(request,
            (GET) (/) => {
                self.handle_home()
            },
            (GET) (/visualize/rbx) => {
                self.handle_visualize_rbx()
            },
            (GET) (/visualize/imfs) => {
                self.handle_visualize_imfs()
            },
            _ => Response::empty_404()
        )
    }

    fn handle_home(&self) -> Response {
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

        Response::html(format!("<!DOCTYPE html>{}", page))
    }

    fn handle_visualize_rbx(&self) -> Response {
        let rbx_session = self.live_session.rbx_session.lock().unwrap();
        let dot_source = format!("{}", VisualizeRbxSession(&rbx_session));

        match graphviz_to_svg(&dot_source) {
            Some(svg) => Response::svg(svg),
            None => Response::text(dot_source),
        }
    }

    fn handle_visualize_imfs(&self) -> Response {
        let imfs = self.live_session.imfs.lock().unwrap();
        let dot_source = format!("{}", VisualizeImfs(&imfs));

        match graphviz_to_svg(&dot_source) {
            Some(svg) => Response::svg(svg),
            None => Response::text(dot_source),
        }
    }
}