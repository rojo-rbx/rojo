mod test_util;

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    thread,
    time::Duration,
};

use log::error;
use tempfile::{tempdir, TempDir};
use pretty_assertions::assert_eq;

use librojo::{
    imfs::Imfs,
    project::Project,
    live_session::LiveSession,
    rbx_snapshot::{SnapshotContext, snapshot_project_tree},
    visualize::{VisualizeRbxTree, graphviz_to_svg},
};

use crate::test_util::{
    copy_recursive,
    snapshot::*,
    tree::trees_equal,
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
    multi_partition_game,
    nested_partitions,
    single_partition_game,
    single_partition_model,
    transmute_partition
);

#[test]
fn multi_partition_game() {
    let _ = env_logger::try_init();
    let source_path = project_path("multi_partition_game");

    let (dir, live_session) = start_session(&source_path);
    tree_step("initial", &live_session, &source_path);

    let added_path = dir.path().join("a/added");
    fs::create_dir_all(&added_path)
        .expect("Couldn't create directory");
    thread::sleep(Duration::from_millis(250));

    tree_step("with_dir", &live_session, &source_path);

    let moved_path = dir.path().join("b/added");
    fs::rename(&added_path, &moved_path)
        .expect("Couldn't rename directory");
    thread::sleep(Duration::from_millis(250));

    tree_step("with_moved_dir", &live_session, &source_path);
}

/// Find the path to the given test project relative to the manifest.
fn project_path(name: &str) -> PathBuf {
    let mut path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-projects");
    path.push(name);
    path
}

/// Starts a new LiveSession for the project located at the given file path.
fn start_session(source_path: &Path) -> (TempDir, LiveSession) {
    let dir = tempdir()
        .expect("Couldn't create temporary directory");

    copy_recursive(&source_path, dir.path())
        .expect("Couldn't copy project to temporary directory");

    let project = Arc::new(Project::load_fuzzy(dir.path())
        .expect("Couldn't load project from temp directory"));

    let live_session = LiveSession::new(Arc::clone(&project))
        .expect("Couldn't start live session");

    (dir, live_session)
}

/// Marks a 'step' in the test, which will snapshot the session's current
/// RbxTree object and compare it against the saved snapshot if it exists.
fn tree_step(step: &str, live_session: &LiveSession, source_path: &Path) {
    let rbx_session = live_session.rbx_session.lock().unwrap();
    let tree = rbx_session.get_tree();

    match read_tree_by_name(source_path, step) {
        Some(expected) => match trees_equal(&expected, tree) {
            Ok(_) => {}
            Err(e) => {
                error!("Trees at step '{}' were not equal.\n{}", step, e);

                let expected_gv = format!("{}", VisualizeRbxTree {
                    tree: &expected,
                    metadata: &HashMap::new(),
                });

                let actual_gv = format!("{}", VisualizeRbxTree {
                    tree: rbx_session.get_tree(),
                    metadata: &HashMap::new(),
                });

                let output_dir = PathBuf::from("failed-snapshots");
                fs::create_dir_all(&output_dir)
                    .expect("Could not create failed-snapshots directory");

                let expected_basename = format!("{}-{}-expected", live_session.root_project().name, step);
                let actual_basename = format!("{}-{}-actual", live_session.root_project().name, step);

                let mut expected_out = output_dir.join(expected_basename);
                let mut actual_out = output_dir.join(actual_basename);

                match (graphviz_to_svg(&expected_gv), graphviz_to_svg(&actual_gv)) {
                    (Some(expected_svg), Some(actual_svg)) => {
                        expected_out.set_extension("svg");
                        actual_out.set_extension("svg");

                        fs::write(&expected_out, expected_svg)
                            .expect("Couldn't write expected SVG");

                        fs::write(&actual_out, actual_svg)
                            .expect("Couldn't write actual SVG");
                    }
                    _ => {
                        expected_out.set_extension("gv");
                        actual_out.set_extension("gv");

                        fs::write(&expected_out, expected_gv)
                            .expect("Couldn't write expected GV");

                        fs::write(&actual_out, actual_gv)
                            .expect("Couldn't write actual GV");
                    }
                }

                error!("Output at {} and {}", expected_out.display(), actual_out.display());

                panic!("Tree mismatch at step '{}'", step);
            }
        }
        None => {
            write_tree_by_name(source_path, step, tree);
        }
    }
}

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