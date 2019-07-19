//! Defines the semantics that Rojo uses to turn entries on the filesystem into
//! Roblox instances using the instance snapshot subsystem.

#![allow(dead_code)]

mod error;
mod context;
mod from_imfs;
mod from_file_txt;