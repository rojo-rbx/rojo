//! Initialization routines that are used by more than one Rojo command or
//! utility.

use std::path::Path;

use rbx_dom_weak::RbxInstanceProperties;

use crate::{
    project::{Project, ProjectLoadError},
    snapshot::{
        apply_patch_set, compute_patch_set, InstanceContext, InstancePropertiesWithMeta, RojoTree,
    },
    snapshot_middleware::snapshot_from_vfs,
    vfs::{Vfs, VfsFetcher},
};

pub fn start<F: VfsFetcher>(
    fuzzy_project_path: &Path,
    vfs: &Vfs<F>,
) -> (Option<Project>, RojoTree) {
    log::trace!("Loading project file from {}", fuzzy_project_path.display());
    let maybe_project = match Project::load_fuzzy(fuzzy_project_path) {
        Ok(project) => Some(project),
        Err(ProjectLoadError::NotFound) => None,
        Err(other) => panic!("{}", other), // TODO: return error upward
    };

    log::trace!("Constructing initial tree");
    let mut tree = RojoTree::new(InstancePropertiesWithMeta {
        properties: RbxInstanceProperties {
            name: "ROOT".to_owned(),
            class_name: "Folder".to_owned(),
            properties: Default::default(),
        },
        metadata: Default::default(),
    });

    let root_id = tree.get_root_id();

    log::trace!("Constructing snapshot context");
    let snapshot_context = InstanceContext::default();
    if let Some(project) = &maybe_project {
        // If the project file defines no plugins, then there's no need to
        // initialize the snapshot plugin context.
        if !project.plugins.is_empty() {
            // TODO: Initialize plugins in instance context
        }
    }

    log::trace!("Reading project root");
    let entry = vfs
        .get(fuzzy_project_path)
        .expect("could not get project path");

    log::trace!("Generating snapshot of instances from VFS");
    let snapshot = snapshot_from_vfs(&snapshot_context, vfs, &entry)
        .expect("snapshot failed")
        .expect("snapshot did not return an instance");

    log::trace!("Computing patch set");
    let patch_set = compute_patch_set(&snapshot, &tree, root_id);

    log::trace!("Applying patch set");
    apply_patch_set(&mut tree, patch_set);

    (maybe_project, tree)
}
