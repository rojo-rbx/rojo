mod test_util;

use std::path::Path;

use pretty_assertions::assert_eq;

use librojo::{
    imfs::Imfs,
    project::Project,
    rbx_snapshot::{SnapshotContext, snapshot_project_tree},
};

use crate::test_util::{
    snapshot::*,
};

macro_rules! generate_snapshot_tests {
    ($($name: ident),*) => {
        $(
            paste::item! {
                #[test]
                fn [<snapshot_ $name>]() {
                    let _ = env_logger::try_init();

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
    fs_project_merging,
    json_model,
    localization,
    multi_partition_game,
    nested_partitions,
    single_partition_game,
    single_partition_model,
    transmute_partition
);

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