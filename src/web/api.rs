//! Defines Rojo's HTTP API, all under /api. These endpoints generally return
//! JSON.

use std::{collections::HashMap, fs, io, io::BufWriter, path::PathBuf, str::FromStr, sync::Arc};

use hyper::{body, Body, Method, Request, Response, StatusCode};
use opener::OpenError;
use rbx_dom_weak::{
    types::{Ref, Variant},
    InstanceBuilder, WeakDom,
};
use roblox_install::RobloxStudio;
use uuid::Uuid;

use crate::{
    serve_session::ServeSession,
    snapshot::{InstanceWithMeta, PatchSet, PatchUpdate},
    web::{
        interface::{
            ErrorResponse, FetchRequest, FetchResponse, Instance, OpenResponse, ReadResponse,
            ServerInfoResponse, SubscribeMessage, SubscribeResponse, WriteRequest, WriteResponse,
            PROTOCOL_VERSION, SERVER_VERSION,
        },
        util::{json, json_ok},
    },
};

const FETCH_DIR_NAME: &str = ".rojo";

pub async fn call(serve_session: Arc<ServeSession>, request: Request<Body>) -> Response<Body> {
    let service = ApiService::new(serve_session);

    log::debug!("{} request received to {}", request.method(), request.uri());
    match (request.method(), request.uri().path()) {
        (&Method::GET, "/api/rojo") => service.handle_api_rojo().await,
        (&Method::GET, path) if path.starts_with("/api/read/") => {
            service.handle_api_read(request).await
        }
        (&Method::GET, path) if path.starts_with("/api/subscribe/") => {
            service.handle_api_subscribe(request).await
        }
        (&Method::POST, path) if path.starts_with("/api/open/") => {
            service.handle_api_open(request).await
        }

        (&Method::POST, "/api/write") => service.handle_api_write(request).await,

        (&Method::POST, path) if path.starts_with("/api/fetch/") => {
            service.handle_api_fetch_get(request).await
        }

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

        json_ok(&WriteResponse { session_id })
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

        json_ok(&OpenResponse {
            session_id: self.serve_session.session_id(),
        })
    }

    async fn handle_api_fetch_get(&self, request: Request<Body>) -> Response<Body> {
        let body = body::to_bytes(request.into_body()).await.unwrap();

        let request: FetchRequest = match serde_json::from_slice(&body) {
            Ok(request) => request,
            Err(err) => {
                return json(
                    ErrorResponse::bad_request(format!("Malformed request body: {}", err)),
                    StatusCode::BAD_REQUEST,
                );
            }
        };

        let content_dir = match RobloxStudio::locate() {
            Ok(path) => path.content_path().to_path_buf(),
            Err(_) => {
                return json(
                    ErrorResponse::internal_error("Cannot locate Studio install"),
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
            }
        };

        let temp_dir = content_dir.join(FETCH_DIR_NAME);
        match fs::create_dir(&temp_dir) {
            // We want to silently move on if the folder already exists
            Err(err) if err.kind() != io::ErrorKind::AlreadyExists => {
                return json(
                    ErrorResponse::internal_error(format!(
                        "Could not create Rojo content directory: {}",
                        &temp_dir.display()
                    )),
                    StatusCode::INTERNAL_SERVER_ERROR,
                );
            }
            _ => {}
        }

        let uuid = Uuid::new_v4();
        let mut file_name = PathBuf::from(uuid.to_string());
        file_name.set_extension("rbxm");

        let out_path = temp_dir.join(&file_name);
        let relative_path = PathBuf::from(FETCH_DIR_NAME).join(file_name);

        let mut writer = BufWriter::new(match fs::File::create(&out_path) {
            Ok(handle) => handle,
            Err(_) => {
                return json(
                    ErrorResponse::internal_error("Could not create temporary file"),
                    StatusCode::INTERNAL_SERVER_ERROR,
                );
            }
        });

        let tree = self.serve_session.tree();
        let inner_tree = tree.inner();
        let mut sub_tree = WeakDom::new(InstanceBuilder::new("Folder"));
        let reify_ref = sub_tree.insert(
            sub_tree.root_ref(),
            InstanceBuilder::new("Folder").with_name("Reified"),
        );
        let map_ref = sub_tree.insert(
            sub_tree.root_ref(),
            InstanceBuilder::new("Folder").with_name("ReferentMap"),
        );
        // Because referents can't be cleanly communicated across a network
        // boundary, we have to get creative. So for every Instance we're
        // building into a model, an ObjectValue is created's named after the
        // old referent and points to the fetched copy of the Instance.
        for referent in request.id_list {
            if inner_tree.get_by_ref(referent).is_some() {
                log::trace!("Creating clone of {referent} into subtree");
                let new_ref = generate_fetch_copy(&inner_tree, &mut sub_tree, reify_ref, referent);
                sub_tree.insert(
                    map_ref,
                    InstanceBuilder::new("ObjectValue")
                        .with_property("Value", Variant::Ref(new_ref))
                        .with_name(referent.to_string()),
                );
            } else {
                return json(
                    ErrorResponse::bad_request("Invalid ID provided to fetch endpoint"),
                    StatusCode::BAD_REQUEST,
                );
            }
        }
        if let Err(_) = rbx_binary::to_writer(&mut writer, &sub_tree, &[sub_tree.root_ref()]) {
            return json(
                ErrorResponse::internal_error("Could not build subtree into model file"),
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
        drop(tree);

        log::debug!("Wrote model file to {}", out_path.display());

        json_ok(FetchResponse {
            session_id: self.serve_session.session_id(),
            path: relative_path.to_string_lossy(),
        })
    }
}

/// If this instance is represented by a script, try to find the correct .lua or .luau
/// file to open to edit it.
fn pick_script_path(instance: InstanceWithMeta<'_>) -> Option<PathBuf> {
    match instance.class_name() {
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

/// Creates a copy of the Instance pointed to by `referent` from `old_tree`,
/// puts it inside of `new_tree`, and parents it to `parent_ref`.
///
/// In the event that the Instance is of a class with special parent
/// requirements such as `Attachment`, it will instead make an Instance
/// of the required parent class and place the cloned Instance as a child
/// of that Instance.
///
/// Regardless, the referent of the clone is returned.
fn generate_fetch_copy(
    old_tree: &WeakDom,
    new_tree: &mut WeakDom,
    parent_ref: Ref,
    referent: Ref,
) -> Ref {
    // We can't use `clone_into_external` here because it also clones the
    // subtree
    let old_inst = old_tree.get_by_ref(referent).unwrap();
    let new_ref = new_tree.insert(
        Ref::none(),
        InstanceBuilder::new(&old_inst.class)
            .with_name(&old_inst.name)
            .with_properties(old_inst.properties.clone()),
    );

    // Certain classes need to have specific parents otherwise Studio
    // doesn't want to load the model.
    let real_parent = match old_inst.class.as_str() {
        // These are services, but they're listed here for posterity.
        "Terrain" | "StarterPlayerScripts" | "StarterCharacterScripts" => parent_ref,

        "Attachment" => new_tree.insert(parent_ref, InstanceBuilder::new("Part")),
        "Bone" => new_tree.insert(parent_ref, InstanceBuilder::new("Part")),
        "Animator" => new_tree.insert(parent_ref, InstanceBuilder::new("Humanoid")),
        "BaseWrap" => new_tree.insert(parent_ref, InstanceBuilder::new("MeshPart")),
        "WrapLayer" => new_tree.insert(parent_ref, InstanceBuilder::new("MeshPart")),
        "WrapTarget" => new_tree.insert(parent_ref, InstanceBuilder::new("MeshPart")),
        _ => parent_ref,
    };
    new_tree.transfer_within(new_ref, real_parent);

    new_ref
}
