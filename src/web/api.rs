//! Defines Rojo's HTTP API, all under /api. These endpoints generally return
//! JSON.

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    str::FromStr,
    sync::Arc,
};

use hyper::{body, Body, Method, Request, Response, StatusCode};
use opener::OpenError;
use rbx_dom_weak::{
    types::{Ref, Variant},
    InstanceBuilder, UstrMap, WeakDom,
};

use crate::{
    serve_session::ServeSession,
    snapshot::{InstanceWithMeta, PatchSet, PatchUpdate},
    web::{
        interface::{
            ErrorResponse, Instance, OpenResponse, ReadResponse, ServerInfoResponse,
            SubscribeMessage, SubscribeResponse, WriteRequest, WriteResponse, PROTOCOL_VERSION,
            SERVER_VERSION,
        },
        util::{json, json_ok},
    },
    web_api::{BufferEncode, InstanceUpdate, RefPatchResponse, SerializeResponse},
};

pub async fn call(serve_session: Arc<ServeSession>, request: Request<Body>) -> Response<Body> {
    let service = ApiService::new(serve_session);

    match (request.method(), request.uri().path()) {
        (&Method::GET, "/api/rojo") => service.handle_api_rojo().await,
        (&Method::GET, path) if path.starts_with("/api/read/") => {
            service.handle_api_read(request).await
        }
        (&Method::GET, path) if path.starts_with("/api/subscribe/") => {
            service.handle_api_subscribe(request).await
        }
        (&Method::GET, path) if path.starts_with("/api/serialize/") => {
            service.handle_api_serialize(request).await
        }
        (&Method::GET, path) if path.starts_with("/api/ref-patch/") => {
            service.handle_api_ref_patch(request).await
        }

        (&Method::POST, path) if path.starts_with("/api/open/") => {
            service.handle_api_open(request).await
        }
        (&Method::POST, "/api/write") => service.handle_api_write(request).await,

        (_method, path) => json(
            ErrorResponse::not_found(format!("Route not found: {}", path)),
            StatusCode::NOT_FOUND,
        ),
    }
}

pub struct ApiService {
    serve_session: Arc<ServeSession>,
}

impl ApiService {
    pub fn new(serve_session: Arc<ServeSession>) -> Self {
        ApiService { serve_session }
    }

    /// Get a summary of information about the server
    async fn handle_api_rojo(&self) -> Response<Body> {
        let tree = self.serve_session.tree();
        let root_instance_id = tree.get_root_id();

        json_ok(&ServerInfoResponse {
            server_version: SERVER_VERSION.to_owned(),
            protocol_version: PROTOCOL_VERSION,
            session_id: self.serve_session.session_id(),
            project_name: self.serve_session.project_name().to_owned(),
            expected_place_ids: self.serve_session.serve_place_ids().cloned(),
            unexpected_place_ids: self.serve_session.blocked_place_ids().cloned(),
            place_id: self.serve_session.place_id(),
            game_id: self.serve_session.game_id(),
            root_instance_id,
        })
    }

    /// Retrieve any messages past the given cursor index, and if
    /// there weren't any, subscribe to receive any new messages.
    async fn handle_api_subscribe(&self, request: Request<Body>) -> Response<Body> {
        let argument = &request.uri().path()["/api/subscribe/".len()..];
        let input_cursor: u32 = match argument.parse() {
            Ok(v) => v,
            Err(err) => {
                return json(
                    ErrorResponse::bad_request(format!("Malformed message cursor: {}", err)),
                    StatusCode::BAD_REQUEST,
                );
            }
        };

        let session_id = self.serve_session.session_id();

        let result = self
            .serve_session
            .message_queue()
            .subscribe(input_cursor)
            .await;

        let tree_handle = self.serve_session.tree_handle();

        match result {
            Ok((message_cursor, messages)) => {
                let tree = tree_handle.lock().unwrap();

                let api_messages = messages
                    .into_iter()
                    .map(|patch| SubscribeMessage::from_patch_update(&tree, patch))
                    .collect();

                json_ok(SubscribeResponse {
                    session_id,
                    message_cursor,
                    messages: api_messages,
                })
            }
            Err(_) => json(
                ErrorResponse::internal_error("Message queue disconnected sender"),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        }
    }

    async fn handle_api_write(&self, request: Request<Body>) -> Response<Body> {
        let session_id = self.serve_session.session_id();
        let tree_mutation_sender = self.serve_session.tree_mutation_sender();

        let body = body::to_bytes(request.into_body()).await.unwrap();

        let request: WriteRequest = match serde_json::from_slice(&body) {
            Ok(request) => request,
            Err(err) => {
                return json(
                    ErrorResponse::bad_request(format!("Invalid body: {}", err)),
                    StatusCode::BAD_REQUEST,
                );
            }
        };

        if request.session_id != session_id {
            return json(
                ErrorResponse::bad_request("Wrong session ID"),
                StatusCode::BAD_REQUEST,
            );
        }

        let updated_instances = request
            .updated
            .into_iter()
            .map(|update| PatchUpdate {
                id: update.id,
                changed_class_name: update.changed_class_name,
                changed_name: update.changed_name,
                changed_properties: update.changed_properties,
                changed_metadata: None,
            })
            .collect();

        tree_mutation_sender
            .send(PatchSet {
                removed_instances: Vec::new(),
                added_instances: Vec::new(),
                updated_instances,
            })
            .unwrap();

        json_ok(WriteResponse { session_id })
    }

    async fn handle_api_read(&self, request: Request<Body>) -> Response<Body> {
        let argument = &request.uri().path()["/api/read/".len()..];
        let requested_ids: Result<Vec<Ref>, _> = argument.split(',').map(Ref::from_str).collect();

        let requested_ids = match requested_ids {
            Ok(ids) => ids,
            Err(_) => {
                return json(
                    ErrorResponse::bad_request("Malformed ID list"),
                    StatusCode::BAD_REQUEST,
                );
            }
        };

        let message_queue = self.serve_session.message_queue();
        let message_cursor = message_queue.cursor();

        let tree = self.serve_session.tree();

        let mut instances = HashMap::new();

        for id in requested_ids {
            if let Some(instance) = tree.get_instance(id) {
                instances.insert(id, Instance::from_rojo_instance(instance));

                for descendant in tree.descendants(id) {
                    instances.insert(descendant.id(), Instance::from_rojo_instance(descendant));
                }
            }
        }

        json_ok(ReadResponse {
            session_id: self.serve_session.session_id(),
            message_cursor,
            instances,
        })
    }

    /// Accepts a list of IDs and returns them serialized as a binary model.
    /// The model is sent in a schema that causes Roblox to deserialize it as
    /// a Luau `buffer`.
    ///
    /// The returned model is a folder that contains ObjectValues with names
    /// that correspond to the requested Instances. These values have their
    /// `Value` property set to point to the requested Instance.
    async fn handle_api_serialize(&self, request: Request<Body>) -> Response<Body> {
        let argument = &request.uri().path()["/api/serialize/".len()..];
        let requested_ids: Result<Vec<Ref>, _> = argument.split(',').map(Ref::from_str).collect();

        let requested_ids = match requested_ids {
            Ok(ids) => ids,
            Err(_) => {
                return json(
                    ErrorResponse::bad_request("Malformed ID list"),
                    StatusCode::BAD_REQUEST,
                );
            }
        };
        let mut response_dom = WeakDom::new(InstanceBuilder::new("Folder"));

        let tree = self.serve_session.tree();
        for id in &requested_ids {
            if let Some(instance) = tree.get_instance(*id) {
                let clone = response_dom.insert(
                    Ref::none(),
                    InstanceBuilder::new(instance.class_name())
                        .with_name(instance.name())
                        .with_properties(instance.properties().clone()),
                );
                let object_value = response_dom.insert(
                    response_dom.root_ref(),
                    InstanceBuilder::new("ObjectValue")
                        .with_name(id.to_string())
                        .with_property("Value", clone),
                );

                let mut child_ref = clone;
                if let Some(parent_class) = parent_requirements(&instance.class_name()) {
                    child_ref =
                        response_dom.insert(object_value, InstanceBuilder::new(parent_class));
                    response_dom.transfer_within(clone, child_ref);
                }

                response_dom.transfer_within(child_ref, object_value);
            } else {
                json(
                    ErrorResponse::bad_request(format!("provided id {id} is not in the tree")),
                    StatusCode::BAD_REQUEST,
                );
            }
        }
        drop(tree);

        let mut source = Vec::new();
        rbx_binary::to_writer(&mut source, &response_dom, &[response_dom.root_ref()]).unwrap();

        json_ok(SerializeResponse {
            session_id: self.serve_session.session_id(),
            model_contents: BufferEncode::new(source),
        })
    }

    /// Returns a list of all referent properties that point towards the
    /// provided IDs. Used because the plugin does not store a RojoTree,
    /// and referent properties need to be updated after the serialize
    /// endpoint is used.
    async fn handle_api_ref_patch(self, request: Request<Body>) -> Response<Body> {
        let argument = &request.uri().path()["/api/ref-patch/".len()..];
        let requested_ids: Result<HashSet<Ref>, _> =
            argument.split(',').map(Ref::from_str).collect();

        let requested_ids = match requested_ids {
            Ok(ids) => ids,
            Err(_) => {
                return json(
                    ErrorResponse::bad_request("Malformed ID list"),
                    StatusCode::BAD_REQUEST,
                );
            }
        };

        let mut instance_updates: HashMap<Ref, InstanceUpdate> = HashMap::new();

        let tree = self.serve_session.tree();
        for instance in tree.descendants(tree.get_root_id()) {
            for (prop_name, prop_value) in instance.properties() {
                let Variant::Ref(prop_value) = prop_value else {
                    continue;
                };
                if let Some(target_id) = requested_ids.get(prop_value) {
                    let instance_id = instance.id();
                    let update =
                        instance_updates
                            .entry(instance_id)
                            .or_insert_with(|| InstanceUpdate {
                                id: instance_id,
                                changed_class_name: None,
                                changed_name: None,
                                changed_metadata: None,
                                changed_properties: UstrMap::default(),
                            });
                    update
                        .changed_properties
                        .insert(*prop_name, Some(Variant::Ref(*target_id)));
                }
            }
        }

        json_ok(RefPatchResponse {
            session_id: self.serve_session.session_id(),
            patch: SubscribeMessage {
                added: HashMap::new(),
                removed: Vec::new(),
                updated: instance_updates.into_values().collect(),
            },
        })
    }

    /// Open a script with the given ID in the user's default text editor.
    async fn handle_api_open(&self, request: Request<Body>) -> Response<Body> {
        let argument = &request.uri().path()["/api/open/".len()..];
        let requested_id = match Ref::from_str(argument) {
            Ok(id) => id,
            Err(_) => {
                return json(
                    ErrorResponse::bad_request("Invalid instance ID"),
                    StatusCode::BAD_REQUEST,
                );
            }
        };

        let tree = self.serve_session.tree();

        let instance = match tree.get_instance(requested_id) {
            Some(instance) => instance,
            None => {
                return json(
                    ErrorResponse::bad_request("Instance not found"),
                    StatusCode::NOT_FOUND,
                );
            }
        };

        let script_path = match pick_script_path(instance) {
            Some(path) => path,
            None => {
                return json(
                    ErrorResponse::bad_request(
                        "No appropriate file could be found to open this script",
                    ),
                    StatusCode::NOT_FOUND,
                );
            }
        };

        match opener::open(&script_path) {
            Ok(()) => {}
            Err(error) => match error {
                OpenError::Io(io_error) => {
                    return json(
                        ErrorResponse::internal_error(format!(
                            "Attempting to open {} failed because of the following io error: {}",
                            script_path.display(),
                            io_error
                        )),
                        StatusCode::INTERNAL_SERVER_ERROR,
                    )
                }
                OpenError::ExitStatus {
                    cmd,
                    status,
                    stderr,
                } => {
                    return json(
                        ErrorResponse::internal_error(format!(
                            r#"The command '{}' to open '{}' failed with the error code '{}'.
                            Error logs:
                            {}"#,
                            cmd,
                            script_path.display(),
                            status,
                            stderr
                        )),
                        StatusCode::INTERNAL_SERVER_ERROR,
                    )
                }
            },
        };

        json_ok(OpenResponse {
            session_id: self.serve_session.session_id(),
        })
    }
}

/// If this instance is represented by a script, try to find the correct .lua or .luau
/// file to open to edit it.
fn pick_script_path(instance: InstanceWithMeta<'_>) -> Option<PathBuf> {
    match instance.class_name().as_str() {
        "Script" | "LocalScript" | "ModuleScript" => {}
        _ => return None,
    }

    // Pick the first listed relevant path that has an extension of .lua or .luau that
    // exists.
    instance
        .metadata()
        .relevant_paths
        .iter()
        .find(|path| {
            // We should only ever open Lua or Luau files to be safe.
            match path.extension().and_then(|ext| ext.to_str()) {
                Some("lua") => {}
                Some("luau") => {}
                _ => return false,
            }

            fs::metadata(path)
                .map(|meta| meta.is_file())
                .unwrap_or(false)
        })
        .map(|path| path.to_owned())
}

/// Certain Instances MUST be a child of specific classes. This function
/// tracks that information for the Serialize endpoint.
///
/// If a parent requirement exists, it will be returned.
/// Otherwise returns `None`.
fn parent_requirements(class: &str) -> Option<&str> {
    Some(match class {
        "Attachment" | "Bone" => "Part",
        "Animator" => "Humanoid",
        "BaseWrap" | "WrapLayer" | "WrapTarget" | "WrapDeformer" => "MeshPart",
        _ => return None,
    })
}
