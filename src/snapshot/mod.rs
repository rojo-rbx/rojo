//! This module defines the instance snapshot subsystem of Rojo.
//!
//! It defines a way to define the instance tree of a project as a pure function
//! of the filesystem by providing a lightweight instance 'snapshot' type, a
//! method to generate minimal patches, and a method that applies those patches.
//!
//! The aim with this approach is to reduce the number of bugs that arise from
//! attempting to manually update instances in response to filesystem updates.
//! Instead of surgically identifying what needs to change, we can do rough
//! "damage-painting", running our relatively fast snapshot function over
//! anything that could have changed and running it through a diffing function
//! to minimize the set of real changes.
//!
//! Building out a snapshot reconciler is mostly overkill for scripts, since
//! their relationships are mostly simple and well-defined. It becomes very
//! important, however, when dealing with large opaque model files and
//! user-defined plugins.

#![allow(dead_code)]

mod instance_snapshot;
mod metadata;
mod patch;
mod patch_apply;
mod patch_compute;
mod tree;

pub use instance_snapshot::InstanceSnapshot;
pub use metadata::*;
pub use patch::*;
pub use patch_apply::apply_patch_set;
pub use patch_compute::compute_patch_set;
pub use tree::*;
