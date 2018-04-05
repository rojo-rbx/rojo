use std::sync::{Arc, Mutex};

use rouille;

use project::Project;
use vfs::{VfsSession};
use middleware::MiddlewareChain;
use web_util::json;

/// The set of configuration the web server needs to start.
pub struct WebConfig {
    pub port: u64,
    pub verbose: bool,
    pub server_id: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServerInfo<'a> {
    server_version: &'static str,
    protocol_version: u64,
    server_id: &'a str,
    project: &'a Project,
    current_time: f64,
}

/// Start the Rojo web server and park our current thread.
pub fn start(config: WebConfig, project: Project, _middleware_chain: &'static MiddlewareChain, vfs: Arc<Mutex<VfsSession>>) {
    let address = format!("localhost:{}", config.port);

    let server_id = config.server_id.to_string();

    rouille::start_server(address, move |request| {
        router!(request,
            (GET) (/) => {
                // Get a summary of information about the server.

                let current_time = {
                    let vfs = vfs.lock().unwrap();

                    vfs.current_time()
                };

                json(ServerInfo {
                    server_version: env!("CARGO_PKG_VERSION"),
                    protocol_version: 1,
                    server_id: &server_id,
                    project: &project,
                    current_time,
                })
            },

            _ => rouille::Response::empty_404()
        )
    });
}
