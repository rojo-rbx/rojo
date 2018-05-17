use std::collections::HashMap;
use std::time::Instant;
use std::sync::{RwLock, Arc};

use rouille;

use web_util::json_response;
use id::Id;
use rbx::RbxInstance;
use rbx_session::RbxSession;

/// The set of configuration the web server needs to start.
pub struct WebConfig {
    pub port: u64,
    pub server_id: u64,
    pub start_time: Instant,
    pub rbx_session: Arc<RwLock<RbxSession>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServerInfoResponse<'a> {
    server_version: &'static str,
    protocol_version: u64,
    server_id: &'a str,
    current_time: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReadAllResponse<'a> {
    server_id: &'a str,
    current_time: f64,
    instances: &'a HashMap<Id, RbxInstance>,
}

/// Start the Rojo web server and park our current thread.
pub fn start(config: WebConfig) {
    let address = format!("localhost:{}", config.port);
    let server_version = env!("CARGO_PKG_VERSION");

    let server_id = config.server_id.to_string();

    rouille::start_server(address, move |request| {
        router!(request,
            (GET) (/) => {
                // Get a summary of information about the server.

                let current_time = {
                    let elapsed = config.start_time.elapsed();

                    elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 / 1_000_000_000.0
                };

                json_response(ServerInfoResponse {
                    server_version,
                    protocol_version: 2,
                    server_id: &server_id,
                    current_time,
                })
            },

            (GET) (/read_all) => {
                let rbx_session = config.rbx_session.read().unwrap();

                let current_time = {
                    let elapsed = config.start_time.elapsed();

                    elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 / 1_000_000_000.0
                };

                json_response(ReadAllResponse {
                    server_id: &server_id,
                    current_time,
                    instances: &rbx_session.instances,
                })
            },

            _ => rouille::Response::empty_404()
        )
    });
}
