// Recursion limit bump is to support Ritz, a JSX-like proc macro used for
// Rojo's web UI currently.
#![recursion_limit = "1024"]

#[macro_use]
mod impl_from;

pub mod commands;

// This module is only public for testing right now, and won't be
// part of the first version of the Rojo API.
#[doc(hidden)]
pub mod project;

#[cfg(test)]
mod tree_view;

mod auth_cookie;
mod change_processor;
mod imfs;
mod message_queue;
mod multimap;
mod path_map;
mod path_serializer;
mod serve_session;
mod session_id;
mod snapshot;
mod snapshot_middleware;
mod web;

pub use crate::session_id::SessionId;
pub use crate::web::interface as web_interface;
