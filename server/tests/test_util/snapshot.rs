use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};

use librojo::{
    project::ProjectNode,
    snapshot_reconciler::RbxSnapshotInstance,
    rbx_session::MetadataPerInstance,
};

const SNAPSHOT_EXPECTED_NAME: &str = "expected-snapshot.json";

/// Snapshots contain absolute paths, which simplifies much of Rojo.
///
/// For saving snapshots to the disk, we should strip off the project folder
/// path to make them machine-independent. This doesn't work for paths that fall
/// outside of the project folder, but that's okay here.
///
/// We also need to sort children, since Rojo tends to enumerate the filesystem
/// in an unpredictable order.
pub fn anonymize_snapshot(project_folder_path: &Path, snapshot: &mut RbxSnapshotInstance) {
    anonymize_metadata(project_folder_path, &mut snapshot.metadata);

    snapshot.children.sort_by(|a, b| a.partial_cmp(b).unwrap());

    for child in snapshot.children.iter_mut() {
        anonymize_snapshot(project_folder_path, child);
    }
}

pub fn anonymize_metadata(project_folder_path: &Path, metadata: &mut MetadataPerInstance) {
    match metadata.source_path.as_mut() {
        Some(path) => *path = anonymize_path(project_folder_path, path),
        None => {},
    }

    match metadata.project_definition.as_mut() {
        Some((_, project_node)) => anonymize_project_node(project_folder_path, project_node),
        None => {},
    }
}

pub fn anonymize_project_node(project_folder_path: &Path, project_node: &mut ProjectNode) {
    match project_node.path.as_mut() {
        Some(path) => *path = anonymize_path(project_folder_path, path),
        None => {},
    }

    for child_node in project_node.children.values_mut() {
        anonymize_project_node(project_folder_path, child_node);
    }
}

pub fn anonymize_path(project_folder_path: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.strip_prefix(project_folder_path)
            .expect("Could not anonymize absolute path")
            .to_path_buf()
    } else {
        path.to_path_buf()
    }
}

pub fn read_expected_snapshot(path: &Path) -> Option<Option<RbxSnapshotInstance<'static>>> {
    let contents = fs::read(path.join(SNAPSHOT_EXPECTED_NAME)).ok()?;
    let snapshot: Option<RbxSnapshotInstance<'static>> = serde_json::from_slice(&contents)
        .expect("Could not deserialize snapshot");

    Some(snapshot)
}

pub fn write_expected_snapshot(path: &Path, snapshot: &Option<RbxSnapshotInstance>) {
    let mut file = File::create(path.join(SNAPSHOT_EXPECTED_NAME))
        .expect("Could not open file to write snapshot");

    serde_json::to_writer_pretty(&mut file, snapshot)
        .expect("Could not serialize snapshot to file");
}