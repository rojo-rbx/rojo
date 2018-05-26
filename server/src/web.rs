use std::collections::HashMap;
use std::sync::{mpsc, RwLock, Arc};

use rouille;

use web_util::json_response;
use id::Id;
use rbx::RbxInstance;
use rbx_session::RbxSession;
use message_session::{MessageSession, Message};

/// The set of configuration the web server needs to start.
pub struct WebConfig {
    pub port: u64,
    pub server_id: u64,
    pub rbx_session: Arc<RwLock<RbxSession>>,
    pub message_session: MessageSession,
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
    message_cursor: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SubscribeResponse<'a> {
    server_id: &'a str,
    messages: &'a [Message],
    message_cursor: i32,
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
                    let messages = config.message_session.messages.read().unwrap();

                    if cursor > messages.len() as i32 {
                        return json_response(SubscribeResponse {
                            server_id: &server_id,
                            messages: &[],
                            message_cursor: messages.len() as i32 - 1,
                        });
                    }

                    if cursor < messages.len() as i32 - 1 {
                        let new_messages = &messages[(cursor + 1) as usize..];
                        let new_cursor = cursor + new_messages.len() as i32;

                        return json_response(SubscribeResponse {
                            server_id: &server_id,
                            messages: new_messages,
                            message_cursor: new_cursor,
                        });
                    }
                }

                let (tx, rx) = mpsc::channel();

                let sender_id = config.message_session.subscribe(tx);

                match rx.recv() {
                    Ok(_) => (),
                    Err(_) => return rouille::Response::text("error!").with_status_code(500),
                }

                config.message_session.unsubscribe(sender_id);

                {
                    let messages = config.message_session.messages.read().unwrap();
                    let new_messages = &messages[(cursor + 1) as usize..];
                    let new_cursor = cursor + new_messages.len() as i32;

                    return json_response(SubscribeResponse {
                        server_id: &server_id,
                        messages: new_messages,
                        message_cursor: new_cursor,
                    });
                }
            },

            (GET) (/read_all) => {
                let rbx_session = config.rbx_session.read().unwrap();

                let message_cursor = {
                    let messages = config.message_session.messages.read().unwrap();
                    messages.len() as i32 - 1
                };

                json_response(ReadAllResponse {
                    server_id: &server_id,
                    message_cursor,
                    instances: &rbx_session.instances,
                })
            },

            // TODO: API to read only specific instances

            _ => rouille::Response::empty_404()
        )
    });
}
