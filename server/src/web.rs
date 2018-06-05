use std::collections::HashMap;
use std::sync::{mpsc, RwLock, Arc};

use rouille::{self, Request, Response};

use id::Id;
use message_session::{MessageSession, Message};
use project::Project;
use rbx::RbxInstance;
use rbx_session::RbxSession;
use session::Session;
use web_util::read_json;

/// The set of configuration the web server needs to start.
pub struct WebConfig {
    pub port: u64,
    pub project: Project,
    pub server_id: u64,
    pub rbx_session: Arc<RwLock<RbxSession>>,
    pub message_session: MessageSession,
}

impl WebConfig {
    pub fn from_session(server_id: u64, port: u64, session: &Session) -> WebConfig {
        WebConfig {
            port,
            server_id,
            project: session.project.clone(),
            rbx_session: session.get_rbx_session(),
            message_session: session.get_message_session(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfoResponse<'a> {
    pub server_id: &'a str,
    pub server_version: &'a str,
    pub protocol_version: u64,
    pub partitions: HashMap<String, Vec<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReadAllResponse<'a> {
    server_id: &'a str,
    message_cursor: i32,
    instances: &'a HashMap<Id, RbxInstance>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReadResponse<'a> {
    server_id: &'a str,
    message_cursor: i32,
    instances: HashMap<Id, &'a RbxInstance>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SubscribeResponse<'a> {
    server_id: &'a str,
    message_cursor: i32,
    messages: &'a [Message],
}

pub struct Server {
    config: WebConfig,
    server_version: &'static str,
    server_id: String,
}

impl Server {
    pub fn new(config: WebConfig) -> Server {
        Server {
            server_version: env!("CARGO_PKG_VERSION"),
            server_id: config.server_id.to_string(),
            config,
        }
    }

    pub fn handle_request(&self, request: &Request) -> Response {
        router!(request,
            (GET) (/) => {
                Response::text("Rojo up and running!")
            },

            (GET) (/api/rojo) => {
                // Get a summary of information about the server.

                let mut partitions = HashMap::new();

                for partition in self.config.project.partitions.values() {
                    partitions.insert(partition.name.clone(), partition.target.clone());
                }

                Response::json(&ServerInfoResponse {
                    server_version: self.server_version,
                    protocol_version: 2,
                    server_id: &self.server_id,
                    partitions: partitions,
                })
            },

            (GET) (/api/subscribe/{ cursor: i32 }) => {
                // Retrieve any messages past the given cursor index, and if
                // there weren't any, subscribe to receive any new messages.

                // Did the client miss any messages since the last subscribe?
                {
                    let messages = self.config.message_session.messages.read().unwrap();

                    if cursor > messages.len() as i32 {
                        return Response::json(&SubscribeResponse {
                            server_id: &self.server_id,
                            messages: &[],
                            message_cursor: messages.len() as i32 - 1,
                        });
                    }

                    if cursor < messages.len() as i32 - 1 {
                        let new_messages = &messages[(cursor + 1) as usize..];
                        let new_cursor = cursor + new_messages.len() as i32;

                        return Response::json(&SubscribeResponse {
                            server_id: &self.server_id,
                            messages: new_messages,
                            message_cursor: new_cursor,
                        });
                    }
                }

                let (tx, rx) = mpsc::channel();

                let sender_id = self.config.message_session.subscribe(tx);

                match rx.recv() {
                    Ok(_) => (),
                    Err(_) => return Response::text("error!").with_status_code(500),
                }

                self.config.message_session.unsubscribe(sender_id);

                {
                    let messages = self.config.message_session.messages.read().unwrap();
                    let new_messages = &messages[(cursor + 1) as usize..];
                    let new_cursor = cursor + new_messages.len() as i32;

                    Response::json(&SubscribeResponse {
                        server_id: &self.server_id,
                        messages: new_messages,
                        message_cursor: new_cursor,
                    })
                }
            },

            (GET) (/api/read_all) => {
                let rbx_session = self.config.rbx_session.read().unwrap();

                let message_cursor = self.config.message_session.get_message_cursor();

                Response::json(&ReadAllResponse {
                    server_id: &self.server_id,
                    message_cursor,
                    instances: rbx_session.tree.get_all_instances(),
                })
            },

            (POST) (/api/read) => {
                let requested_ids = match read_json::<Vec<Id>>(request) {
                    Some(body) => body,
                    None => return rouille::Response::text("Malformed JSON").with_status_code(400),
                };

                let rbx_session = self.config.rbx_session.read().unwrap();

                let message_cursor = self.config.message_session.get_message_cursor();

                let mut instances = HashMap::new();

                for requested_id in &requested_ids {
                    rbx_session.tree.get_instance(*requested_id, &mut instances);
                }

                Response::json(&ReadResponse {
                    server_id: &self.server_id,
                    message_cursor,
                    instances,
                })
            },

            _ => Response::empty_404()
        )
    }
}

/// Start the Rojo web server, taking over the current thread.
#[allow(unreachable_code)]
pub fn start(config: WebConfig) {
    let address = format!("localhost:{}", config.port);
    let server = Server::new(config);

    rouille::start_server(address, move |request| server.handle_request(request));
}
