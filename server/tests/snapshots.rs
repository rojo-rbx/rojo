use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};

use pretty_assertions::assert_eq;

use librojo::{
    imfs::Imfs,
    project::{Project, ProjectNode},
    rbx_snapshot::{SnapshotContext, snapshot_project_tree},
    snapshot_reconciler::{RbxSnapshotInstance},
};

macro_rules! generate_snapshot_tests {
    ($($name: ident),*) => {
        $(
            paste::item! {
                #[test]
                fn [<snapshot_ $name>]() {
                    let tests_folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-projects");
                    let project_folder = tests_folder.join(stringify!($name));
                    run_snapshot_test(&project_folder);
                }
            }
        )*
    };
}

generate_snapshot_tests!(
    empty,
    nested_partitions,
    single_partition_game,
    single_partition_model,
    transmute_partition
);

const SNAPSHOT_EXPECTED_NAME: &str = "expected-snapshot.json";

fn run_snapshot_test(path: &Path) {
    println!("Running snapshot from project: {}", path.display());

    let project = Project::load_fuzzy(path)
        .expect("Couldn't load project file for snapshot test");

    let mut imfs = Imfs::new();
    imfs.add_roots_from_project(&project)
        .expect("Could not add IMFS roots to snapshot project");

    let context = SnapshotContext {
        plugin_context: None,
    };

    let mut snapshot = snapshot_project_tree(&context, &imfs, &project)
        .expect("Could not generate snapshot for snapshot test");

    if let Some(snapshot) = snapshot.as_mut() {
        anonymize_snapshot(path, snapshot);
    }

    match read_expected_snapshot(path) {
        Some(expected_snapshot) => assert_eq!(snapshot, expected_snapshot),
        None => write_expected_snapshot(path, &snapshot),
    }
}

/// Snapshots contain absolute paths, which simplifies much of Rojo.
///
/// For saving snapshots to the disk, we should strip off the project folder
/// path to make them machine-independent. This doesn't work for paths that fall
/// outside of the project folder, but that's okay here.
///
/// We also need to sort children, since Rojo tends to enumerate the filesystem
/// in an unpredictable order.
fn anonymize_snapshot(project_folder_path: &Path, snapshot: &mut RbxSnapshotInstance) {
    match snapshot.metadata.source_path.as_mut() {
        Some(path) => *path = anonymize_path(project_folder_path, path),
        None => {},
    }

    match snapshot.metadata.project_definition.as_mut() {
        Some((_, project_node)) => anonymize_project_node(project_folder_path, project_node),
        None => {},
    }

    snapshot.children.sort_by(|a, b| a.partial_cmp(b).unwrap());

    for child in snapshot.children.iter_mut() {
        anonymize_snapshot(project_folder_path, child);
    }
}

fn anonymize_project_node(project_folder_path: &Path, project_node: &mut ProjectNode) {
    match project_node.path.as_mut() {
        Some(path) => *path = anonymize_path(project_folder_path, path),
        None => {},
    }

    for child_node in project_node.children.values_mut() {
        anonymize_project_node(project_folder_path, child_node);
    }
}

fn anonymize_path(project_folder_path: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.strip_prefix(project_folder_path)
            .expect("Could not anonymize absolute path")
            .to_path_buf()
    } else {
        path.to_path_buf()
    }
}

fn read_expected_snapshot(path: &Path) -> Option<Option<RbxSnapshotInstance<'static>>> {
    let contents = fs::read(path.join(SNAPSHOT_EXPECTED_NAME)).ok()?;
    let snapshot: Option<RbxSnapshotInstance<'static>> = serde_json::from_slice(&contents)
        .expect("Could not deserialize snapshot");

    Some(snapshot)
}

fn write_expected_snapshot(path: &Path, snapshot: &Option<RbxSnapshotInstance>) {
    let mut file = File::create(path.join(SNAPSHOT_EXPECTED_NAME))
        .expect("Could not open file to write snapshot");

    serde_json::to_writer_pretty(&mut file, snapshot)
        .expect("Could not serialize snapshot to file");
}