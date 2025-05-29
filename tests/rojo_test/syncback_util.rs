use std::{fs, path::Path, process::Command};

use insta::assert_snapshot;
use tempfile::tempdir;

use crate::rojo_test::io_util::SYNCBACK_TESTS_PATH;

use super::io_util::{copy_recursive, get_working_dir_path, ROJO_PATH};

const INPUT_FILE_PROJECT: &str = "input-project";
const INPUT_FILE: &str = "input.rbxl";

/// Convenience method to run a `rojo syncback` test.
///
/// Test projects should be defined in the `syncback-tests` folder; their filename
/// should be given as the first parameter.
///
/// The passed in callback is where the actual test body should go. Setup and
/// cleanup happens automatically.
pub fn run_syncback_test(name: &str, callback: impl FnOnce(&Path)) {
    let _ = env_logger::try_init();

    let working_dir = get_working_dir_path();

    let source_path = Path::new(SYNCBACK_TESTS_PATH)
        .join(name)
        .join(INPUT_FILE_PROJECT);
    let input_file = Path::new(SYNCBACK_TESTS_PATH).join(name).join(INPUT_FILE);

    let test_dir = tempdir().expect("Couldn't create temporary directory");
    let project_path = test_dir
        .path()
        .canonicalize()
        .expect("Couldn't canonicalize temporary directory path")
        .join(name);

    let source_is_file = fs::metadata(&source_path).unwrap().is_file();

    if source_is_file {
        fs::copy(&source_path, &project_path).expect("couldn't copy project file");
    } else {
        fs::create_dir(&project_path).expect("Couldn't create temporary project subdirectory");

        copy_recursive(&source_path, &project_path)
            .expect("Couldn't copy project to temporary directory");
    };

    let output = Command::new(ROJO_PATH)
        .current_dir(working_dir)
        .args([
            "--color",
            "never",
            "syncback",
            project_path.to_str().unwrap(),
            "--input",
            input_file.to_str().unwrap(),
            "--non-interactive",
            "--list",
        ])
        .output()
        .expect("Couldn't spawn syncback process");

    assert!(output.status.success(), "Rojo did not exit correctly");

    let mut settings = insta::Settings::new();
    let snapshot_path = Path::new(SYNCBACK_TESTS_PATH)
        .parent()
        .unwrap()
        .join("syncback-test-snapshots");

    settings.bind(|| {
        assert_snapshot!(
            format!("{name}-stderr"),
            String::from_utf8_lossy(&output.stderr)
        )
    });

    settings.set_snapshot_path(snapshot_path);
    settings.set_sort_maps(true);
    settings.bind(|| callback(project_path.as_path()))
}
