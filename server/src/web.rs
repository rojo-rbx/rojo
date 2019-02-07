//! Defines Rojo's web interface that all clients use to communicate with a
//! running live-sync session.

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    sync::{mpsc, Arc},
};

use serde_derive::{Serialize, Deserialize};
use log::trace;
use rouille::{
    self,
    router,
    Request,
    Response,
};
use rbx_tree::{RbxId, RbxInstance};

use crate::{
    live_session::LiveSession,
    session_id::SessionId,
    snapshot_reconciler::InstanceChanges,
    visualize::{VisualizeRbxSession, VisualizeImfs, graphviz_to_svg},
    rbx_session::{MetadataPerInstance},
};

static HOME_CONTENT: &str = include_str!("../assets/index.html");

/// Contains the instance metadata relevant to Rojo clients.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicInstanceMetadata {
    ignore_unknown_instances: bool,
}

impl PublicInstanceMetadata {
    pub fn from_session_metadata(meta: &MetadataPerInstance) -> PublicInstanceMetadata {
        PublicInstanceMetadata {
            ignore_unknown_instances: meta.ignore_unknown_instances,
        }
    }
}

/// Used to attach metadata specific to Rojo to instances, which come from the
/// rbx_tree crate.
///
/// Both fields are wrapped in Cow in order to make owned-vs-borrowed simpler
/// for tests.
#[derive(Debug, Serialize, Deserialize)]
pub struct InstanceWithMetadata<'a> {
    #[serde(flatten)]
    pub instance: Cow<'a, RbxInstance>,

    #[serde(rename = "Metadata")]
    pub metadata: Option<PublicInstanceMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfoResponse<'a> {
    pub session_id: SessionId,
    pub server_version: &'a str,
    pub protocol_version: u64,
    pub expected_place_ids: Option<HashSet<u64>>,
    pub root_instance_id: RbxId,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadResponse<'a> {
    pub session_id: SessionId,
    pub message_cursor: u32,
    pub instances: HashMap<RbxId, InstanceWithMetadata<'a>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeResponse<'a> {
    pub session_id: SessionId,
    pub message_cursor: u32,
    pub messages: Cow<'a, [InstanceChanges]>,
}

pub struct Server {
    live_session: Arc<LiveSession>,
    server_version: &'static str,
}

impl Server {
    pub fn new(live_session: Arc<LiveSession>) -> Server {
        Server {
            live_session,
            server_version: env!("CARGO_PKG_VERSION"),
        }
    }

    #[allow(unreachable_code)]
    pub fn handle_request(&self, request: &Request) -> Response {
        trace!("Request {} {}", request.method(), request.url());

        router!(request,
            (GET) (/) => {
                self.handle_home()
            },
            (GET) (/api/rojo) => {
                self.handle_api_rojo()
            },
            (GET) (/api/subscribe/{ cursor: u32 }) => {
                self.handle_api_subscribe(cursor)
            },
            (GET) (/api/read/{ id_list: String }) => {
                let requested_ids: Option<Vec<RbxId>> = id_list
                    .split(',')
                    .map(RbxId::parse_str)
                    .collect();

                self.handle_api_read(requested_ids)
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

    pub fn listen(self, port: u16) {
        let address = format!("0.0.0.0:{}", port);

        rouille::start_server(address, move |request| self.handle_request(request));
    }

    fn handle_home(&self) -> Response {
        Response::html(HOME_CONTENT)
    }

    /// Get a summary of information about the server
    fn handle_api_rojo(&self) -> Response {
        let rbx_session = self.live_session.rbx_session.lock().unwrap();
        let tree = rbx_session.get_tree();

        Response::json(&ServerInfoResponse {
            server_version: self.server_version,
            protocol_version: 2,
            session_id: self.live_session.session_id,
            expected_place_ids: self.live_session.project.serve_place_ids.clone(),
            root_instance_id: tree.get_root_id(),
        })
    }

    /// Retrieve any messages past the given cursor index, and if
    /// there weren't any, subscribe to receive any new messages.
    fn handle_api_subscribe(&self, cursor: u32) -> Response {
        let message_queue = Arc::clone(&self.live_session.message_queue);

        // Did the client miss any messages since the last subscribe?
        {
            let (new_cursor, new_messages) = message_queue.get_messages_since(cursor);

            if !new_messages.is_empty() {
                return Response::json(&SubscribeResponse {
                    session_id: self.live_session.session_id,
                    messages: Cow::Borrowed(&new_messages),
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
                session_id: self.live_session.session_id,
                messages: Cow::Owned(new_messages),
                message_cursor: new_cursor,
            })
        }
    }

    fn handle_api_read(&self, requested_ids: Option<Vec<RbxId>>) -> Response {
        let message_queue = Arc::clone(&self.live_session.message_queue);

        let requested_ids = match requested_ids {
            Some(id) => id,
            None => return rouille::Response::text("Malformed ID list").with_status_code(400),
        };

        let rbx_session = self.live_session.rbx_session.lock().unwrap();
        let tree = rbx_session.get_tree();

        let message_cursor = message_queue.get_message_cursor();

        let mut instances = HashMap::new();

        for &requested_id in &requested_ids {
            if let Some(instance) = tree.get_instance(requested_id) {
                let metadata = rbx_session.get_instance_metadata(requested_id)
                    .map(PublicInstanceMetadata::from_session_metadata);

                instances.insert(instance.get_id(), InstanceWithMetadata {
                    instance: Cow::Borrowed(instance),
                    metadata,
                });

                for descendant in tree.descendants(requested_id) {
                    let descendant_meta = rbx_session.get_instance_metadata(descendant.get_id())
                        .map(PublicInstanceMetadata::from_session_metadata);

                    instances.insert(descendant.get_id(), InstanceWithMetadata {
                        instance: Cow::Borrowed(descendant),
                        metadata: descendant_meta,
                    });
                }
            }
        }

        Response::json(&ReadResponse {
            session_id: self.live_session.session_id,
            message_cursor,
            instances,
        })
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