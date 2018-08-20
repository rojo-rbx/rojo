use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{mpsc, Arc};

use rouille::{self, Request, Response};
use rand;

use ::{
    id::Id,
    message_session::Message,
    project::Project,
    rbx::RbxInstance,
    serve_session::ServeSession,
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
    // pub instances: HashMap<Id, Cow<'a, RbxInstance>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeResponse<'a> {
    pub server_id: &'a str,
    pub message_cursor: u32,
    pub messages: Cow<'a, [Message]>,
}

pub struct Server {
    session: Arc<ServeSession>,
    server_version: &'static str,
    server_id: String,
}

impl Server {
    pub fn new(session: Arc<ServeSession>) -> Server {
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

                Response::json(&ServerInfoResponse {
                    server_version: self.server_version,
                    protocol_version: 2,
                    server_id: &self.server_id,
                    root_instance_id: self.session.get_tree().root_instance_id,
                })
            },

            (GET) (/api/subscribe/{ cursor: u32 }) => {
                // Retrieve any messages past the given cursor index, and if
                // there weren't any, subscribe to receive any new messages.

                // Did the client miss any messages since the last subscribe?
                {
                    let (new_cursor, new_messages) = self.session.get_messages().get_messages_since(cursor);

                    if new_messages.len() > 0 {
                        return Response::json(&SubscribeResponse {
                            server_id: &self.server_id,
                            messages: Cow::Borrowed(&[]),
                            message_cursor: new_cursor,
                        })
                    }
                }

                let (tx, rx) = mpsc::channel();

                let sender_id = self.session.get_messages().subscribe(tx);

                match rx.recv() {
                    Ok(_) => (),
                    Err(_) => return Response::text("error!").with_status_code(500),
                }

                self.session.get_messages().unsubscribe(sender_id);

                {
                    let (new_cursor, new_messages) = self.session.get_messages().get_messages_since(cursor);

                    return Response::json(&SubscribeResponse {
                        server_id: &self.server_id,
                        messages: Cow::Owned(new_messages),
                        message_cursor: new_cursor,
                    })
                }
            },

            (GET) (/api/read/{ id_list: String }) => {
                let requested_ids: Result<Vec<Id>, _> = id_list
                    .split(",")
                    .map(str::parse)
                    .collect();

                let requested_ids = match requested_ids {
                    Ok(id) => id,
                    Err(_) => return rouille::Response::text("Malformed ID list").with_status_code(400),
                };

                let rbx_tree = self.session.get_tree();

                let message_cursor = self.session.get_messages().get_message_cursor();

                let mut instances = HashMap::new();

                for &requested_id in &requested_ids {
                    match rbx_tree.get_instance(requested_id) {
                        Some(instance) => {
                            instances.insert(instance.get_id(), instance);

                            for descendant in rbx_tree.iter_descendants(requested_id) {
                                instances.insert(descendant.get_id(), descendant);
                            }
                        },
                        None => {},
                    }
                }

                Response::json(&ReadResponse {
                    server_id: &self.server_id,
                    message_cursor,
                    // instances,
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