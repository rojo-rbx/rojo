//! Initialization routines that are used by more than one Rojo command or
//! utility.

use std::path::Path;

use rbx_dom_weak::RbxInstanceProperties;

use crate::{
    project::Project,
    snapshot::{
        apply_patch_set, compute_patch_set, InstanceContext, InstancePropertiesWithMeta,
        PathIgnoreRule, RojoTree,
    },
    snapshot_middleware::snapshot_from_vfs,
    vfs::{Vfs, VfsFetcher},
};

pub fn start<F: VfsFetcher>(
    fuzzy_project_path: &Path,
    vfs: &Vfs<F>,
) -> (Option<Project>, RojoTree) {
    log::trace!("Loading project file from {}", fuzzy_project_path.display());
    let maybe_project = Project::load_fuzzy(fuzzy_project_path).expect("TODO: Project load failed");

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

    log::trace!("Reading project root");
    let entry = vfs
        .get(fuzzy_project_path)
        .expect("could not get project path");

    let mut instance_context = InstanceContext::default();

    if let Some(project) = &maybe_project {
        let project_folder = project.folder_location();

        let rules = project.glob_ignore_paths.iter().map(|glob| PathIgnoreRule {
            glob: glob.clone(),
            base_path: project_folder.to_path_buf(),
        });

        instance_context.add_path_ignore_rules(rules);
    }

    log::trace!("Generating snapshot of instances from VFS");
    let snapshot = snapshot_from_vfs(&instance_context, vfs, &entry)
        .expect("snapshot failed")
        .expect("snapshot did not return an instance");

    log::trace!("Computing patch set");
    let patch_set = compute_patch_set(&snapshot, &tree, root_id);

    log::trace!("Applying patch set");
    apply_patch_set(&mut tree, patch_set);

    (maybe_project, tree)
}
