use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{mpsc, Arc};

use rouille::{self, Request, Response};
use rand;

use ::{
    id::Id,
    message_queue::Message,
    project::Project,
    rbx::RbxInstance,
    session::Session,
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfoResponse<'a> {
    pub server_id: &'a str,
    pub server_version: &'a str,
    pub protocol_version: u64,
    pub root_instance_id: Id,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadResponse<'a> {
    pub server_id: &'a str,
    pub message_cursor: u32,
    pub instances: HashMap<Id, Cow<'a, RbxInstance>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeResponse<'a> {
    pub server_id: &'a str,
    pub message_cursor: u32,
    pub messages: Cow<'a, [Message]>,
}

pub struct Server {
    session: Arc<Session>,
    server_version: &'static str,
    server_id: String,
}

impl Server {
    pub fn new(session: Arc<Session>) -> Server {
        let server_id = rand::random::<u64>().to_string();

        Server {
            session: session,
            server_version: env!("CARGO_PKG_VERSION"),
            server_id,
        }
    }

    #[allow(unreachable_code)]
    pub fn handle_request(&self, request: &Request) -> Response {
        router!(request,
            (GET) (/) => {
                Response::text("Rojo is up and running!")
            },

            (GET) (/api/rojo) => {
                // Get a summary of information about the server.

                let tree = self.session.tree.read().unwrap();

                Response::json(&ServerInfoResponse {
                    server_version: self.server_version,
                    protocol_version: 2,
                    server_id: &self.server_id,
                    root_instance_id: tree.root_instance_id,
                })
            },

            (GET) (/api/subscribe/{ cursor: u32 }) => {
                // Retrieve any messages past the given cursor index, and if
                // there weren't any, subscribe to receive any new messages.

                let message_queue = Arc::clone(&self.session.message_queue);

                // Did the client miss any messages since the last subscribe?
                {
                    let (new_cursor, new_messages) = message_queue.get_messages_since(cursor);

                    if new_messages.len() > 0 {
                        return Response::json(&SubscribeResponse {
                            server_id: &self.server_id,
                            messages: Cow::Borrowed(&[]),
                            message_cursor: new_cursor,
                        })
                    }
                }

                let (tx, rx) = mpsc::channel();

                let sender_id = message_queue.subscribe(tx);

                match rx.recv() {
                    Ok(_) => (),
                    Err(_) => return Response::text("error!").with_status_code(500),
                }

                message_queue.unsubscribe(sender_id);

                {
                    let (new_cursor, new_messages) = message_queue.get_messages_since(cursor);

                    return Response::json(&SubscribeResponse {
                        server_id: &self.server_id,
                        messages: Cow::Owned(new_messages),
                        message_cursor: new_cursor,
                    })
                }
            },

            (GET) (/api/read/{ id_list: String }) => {
                let message_queue = Arc::clone(&self.session.message_queue);

                let requested_ids: Result<Vec<Id>, _> = id_list
                    .split(",")
                    .map(str::parse)
                    .collect();

                let requested_ids = match requested_ids {
                    Ok(id) => id,
                    Err(_) => return rouille::Response::text("Malformed ID list").with_status_code(400),
                };

                let tree = self.session.tree.read().unwrap();

                let message_cursor = message_queue.get_message_cursor();

                let mut instances = HashMap::new();

                for &requested_id in &requested_ids {
                    match tree.get_instance(requested_id) {
                        Some(instance) => {
                            instances.insert(instance.get_id(), Cow::Borrowed(instance));

                            for descendant in tree.iter_descendants(requested_id) {
                                instances.insert(descendant.get_id(), Cow::Borrowed(descendant));
                            }
                        },
                        None => {},
                    }
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

    pub fn listen(self, port: u64) {
        let address = format!("localhost:{}", port);

        rouille::start_server(address, move |request| self.handle_request(request));
    }
}