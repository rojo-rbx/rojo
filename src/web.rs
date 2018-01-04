use std::io::Read;
use std::sync::{Arc, Mutex};

use rouille;
use serde;
use serde_json;

use project::Project;
use vfs::{VfsSession, VfsChange};
use rbx::RbxInstance;
use plugin::PluginChain;

static MAX_BODY_SIZE: usize = 25 * 1024 * 1024; // 25 MiB

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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReadResult<'a> {
    items: Vec<Option<RbxInstance>>,
    server_id: &'a str,
    current_time: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChangesResult<'a> {
    changes: &'a [VfsChange],
    server_id: &'a str,
    current_time: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WriteSpecifier {
    route: String,
    item: RbxInstance,
}

fn json<T: serde::Serialize>(value: T) -> rouille::Response {
    let data = serde_json::to_string(&value).unwrap();
    rouille::Response::from_data("application/json", data)
}

/// Pulls text that may be JSON out of a Rouille Request object.
///
/// Doesn't do any actual parsing -- all this method does is verify the content
/// type of the request and read the request's body.
fn read_json_text(request: &rouille::Request) -> Option<String> {
    // Bail out if the request body isn't marked as JSON
    match request.header("Content-Type") {
        Some(header) => if !header.starts_with("application/json") {
            return None;
        },
        None => return None,
    }

    let body = match request.data() {
        Some(v) => v,
        None => return None,
    };

    // Allocate a buffer and read up to MAX_BODY_SIZE+1 bytes into it.
    let mut out = Vec::new();
    match body.take(MAX_BODY_SIZE.saturating_add(1) as u64).read_to_end(&mut out) {
        Ok(_) => {},
        Err(_) => return None,
    }

    // If the body was too big (MAX_BODY_SIZE+1), we abort instead of trying to
    // process it.
    if out.len() > MAX_BODY_SIZE {
        return None;
    }

    let parsed = match String::from_utf8(out) {
        Ok(v) => v,
        Err(_) => return None,
    };

    Some(parsed)
}

/// Reads the body out of a Rouille Request and attempts to turn it into JSON.
fn read_json<T>(request: &rouille::Request) -> Option<T>
where
    T: serde::de::DeserializeOwned,
{
    let body = match read_json_text(&request) {
        Some(v) => v,
        None => return None,
    };

    let parsed = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => return None,
    };

    // TODO: Change return type to some sort of Result

    Some(parsed)
}

/// Start the Rojo web server and park our current thread.
pub fn start(config: WebConfig, project: Project, plugin_chain: &'static PluginChain, vfs: Arc<Mutex<VfsSession>>) {
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

            (GET) (/changes/{ last_time: f64 }) => {
                // Get the list of changes since the given time.

                let vfs = vfs.lock().unwrap();
                let current_time = vfs.current_time();
                let changes = vfs.changes_since(last_time);

                json(ChangesResult {
                    changes,
                    server_id: &server_id,
                    current_time,
                })
            },

            (POST) (/read) => {
                // Read some instances from the server according to a JSON
                // format body.

                let read_request: Vec<Vec<String>> = match read_json(&request) {
                    Some(v) => v,
                    None => return rouille::Response::empty_400(),
                };

                // Read the files off of the filesystem that the client
                // requested.
                let (items, current_time) = {
                    let vfs = vfs.lock().unwrap();

                    let current_time = vfs.current_time();

                    let mut items = Vec::new();

                    for route in &read_request {
                        match vfs.read(&route) {
                            Ok(v) => items.push(Some(v)),
                            Err(_) => items.push(None),
                        }
                    }

                    (items, current_time)
                };

                // Transform all of our VfsItem objects into Roblox instances
                // the client can use.
                let rbx_items = items
                    .iter()
                    .map(|item| {
                        match *item {
                            Some(ref item) => plugin_chain.transform_file(item),
                            None => None,
                        }
                    })
                    .collect::<Vec<_>>();

                if config.verbose {
                    println!("Got read request: {:?}", read_request);
                    println!("Responding with:\n\t{:?}", rbx_items);
                }

                json(ReadResult {
                    server_id: &server_id,
                    items: rbx_items,
                    current_time,
                })
            },

            (POST) (/write) => {
                // Not yet implemented.

                let _write_request: Vec<WriteSpecifier> = match read_json(&request) {
                    Some(v) => v,
                    None => return rouille::Response::empty_400(),
                };

                rouille::Response::empty_404()
            },

            _ => rouille::Response::empty_404()
        )
    });
}
