use std::time::Instant;

use rouille;

use project::Project;
use web_util::json_response;

/// The set of configuration the web server needs to start.
pub struct WebConfig {
    pub port: u64,
    pub server_id: u64,
    pub project: Project,
    pub start_time: Instant,
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
pub fn start(config: WebConfig) {
    let address = format!("localhost:{}", config.port);

    let server_id = config.server_id.to_string();

    rouille::start_server(address, move |request| {
        router!(request,
            (GET) (/) => {
                // Get a summary of information about the server.

                let current_time = {
                    let elapsed = config.start_time.elapsed();

                    elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 / 1_000_000_000.0
                };

                json_response(ServerInfo {
                    server_version: env!("CARGO_PKG_VERSION"),
                    protocol_version: 1,
                    server_id: &server_id,
                    project: &config.project,
                    current_time,
                })
            },

            _ => rouille::Response::empty_404()
        )
    });
}
