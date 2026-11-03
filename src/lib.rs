// Recursion limit bump is to support Ritz, a JSX-like proc macro used for
// Rojo's web UI currently.
#![recursion_limit = "1024"]

pub mod cli;

#[cfg(test)]
mod tree_view;

mod change_processor;
mod glob;
mod json;
mod lua_ast;
mod message_queue;
mod multimap;
mod path_serializer;
mod project;
mod resolution;
mod rojo_ref;
mod serve_session;
mod session_id;
mod snapshot;
mod snapshot_middleware;
mod syncback;
mod variant_eq;
mod web;

// TODO: Work out what we should expose publicly

pub use project::*;
pub use rojo_ref::*;
pub use session_id::SessionId;
pub use snapshot::{
    InstanceContext, InstanceMetadata, InstanceSnapshot, InstanceWithMeta, InstanceWithMetaMut,
    RojoDescendants, RojoTree,
};
pub use snapshot_middleware::{snapshot_from_vfs, Middleware, ScriptType};
pub use syncback::{syncback_loop, FsSnapshot, SyncbackData, SyncbackSnapshot};
pub use web::interface as web_api;
