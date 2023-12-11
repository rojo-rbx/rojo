// Recursion limit bump is to support Ritz, a JSX-like proc macro used for
// Rojo's web UI currently.
#![recursion_limit = "1024"]

pub mod cli;

#[cfg(test)]
mod tree_view;

mod auth_cookie;
mod change_processor;
mod glob;
mod lua_ast;
mod message_queue;
mod multimap;
mod path_serializer;
mod project;
mod resolution;
mod serve_session;
mod session_id;
mod snapshot;
mod snapshot_middleware;
mod syncback;
mod web;

pub use project::*;
pub use session_id::SessionId;
pub use web::interface as web_api;
