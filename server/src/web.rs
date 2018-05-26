use std::collections::HashMap;
use std::sync::{mpsc, RwLock, Mutex, Arc};

use rouille;

use web_util::json_response;
use id::{Id, get_id};
use rbx::RbxInstance;
use rbx_session::RbxSession;
use session::SessionEvent;

/// The set of configuration the web server needs to start.
pub struct WebConfig {
    pub port: u64,
    pub server_id: u64,
    pub rbx_session: Arc<RwLock<RbxSession>>,
    pub events: Arc<RwLock<Vec<SessionEvent>>>,
    pub event_listeners: Arc<Mutex<HashMap<Id, mpsc::Sender<()>>>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServerInfoResponse<'a> {
    server_version: &'static str,
    protocol_version: u64,
    server_id: &'a str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReadAllResponse<'a> {
    server_id: &'a str,
    instances: &'a HashMap<Id, RbxInstance>,
    event_cursor: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SubscribeResponse<'a> {
    server_id: &'a str,
    events: &'a [SessionEvent],
    event_cursor: i32,
}

/// Start the Rojo web server and park our current thread.
#[allow(unreachable_code)]
pub fn start(config: WebConfig) {
    let address = format!("localhost:{}", config.port);
    let server_version = env!("CARGO_PKG_VERSION");

    let server_id = config.server_id.to_string();

    rouille::start_server(address, move |request| {
        router!(request,
            (GET) (/) => {
                // Get a summary of information about the server.

                json_response(ServerInfoResponse {
                    server_version,
                    protocol_version: 2,
                    server_id: &server_id,
                })
            },

            (GET) (/subscribe/{ cursor: i32 }) => {
                // Retrieve any messages past the given cursor index, and if
                // there weren't any, subscribe to receive any new messages.

                // Did the client miss any messages since the last subscribe?
                {
                    let events = config.events.read().unwrap();
                    if cursor < events.len() as i32 - 1 {
                        let new_events = &events[(cursor + 1) as usize..];
                        let new_cursor = cursor + new_events.len() as i32;

                        return json_response(SubscribeResponse {
                            server_id: &server_id,
                            events: new_events,
                            event_cursor: new_cursor,
                        });
                    }
                }

                let sender_id = get_id();
                let (tx, rx) = mpsc::channel();

                {
                    let mut event_listeners = config.event_listeners.lock().unwrap();
                    event_listeners.insert(sender_id, tx);
                }

                match rx.recv() {
                    Ok(_) => (),
                    Err(_) => return rouille::Response::text("error!").with_status_code(500),
                }

                {
                    let mut event_listeners = config.event_listeners.lock().unwrap();
                    event_listeners.remove(&sender_id);
                }

                {
                    let events = config.events.read().unwrap();
                    let new_events = &events[(cursor + 1) as usize..];
                    let new_cursor = cursor + new_events.len() as i32;

                    return json_response(SubscribeResponse {
                        server_id: &server_id,
                        events: new_events,
                        event_cursor: new_cursor,
                    });
                }
            },

            (GET) (/read_all) => {
                let rbx_session = config.rbx_session.read().unwrap();

                let event_cursor = {
                    let events = config.events.read().unwrap();
                    events.len() as i32 - 1
                };

                json_response(ReadAllResponse {
                    server_id: &server_id,
                    event_cursor,
                    instances: &rbx_session.instances,
                })
            },

            _ => rouille::Response::empty_404()
        )
    });
}
