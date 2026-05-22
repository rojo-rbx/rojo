use std::{io::Write as _, path::Path, process::Command};

use insta::{assert_snapshot, assert_yaml_snapshot};
use tempfile::tempdir;

use crate::rojo_test::io_util::SYNCBACK_TESTS_PATH;

use super::io_util::{copy_recursive, ROJO_PATH};

const INPUT_FILE_PROJECT: &str = "input-project";
const INPUT_FILE_PLACE: &str = "input.rbxl";
const INPUT_FILE_MODEL: &str = "input.rbxm";

/// Convenience method to run a `rojo syncback` test.
///
/// Test projects should be defined in the `syncback-tests` folder; their filename
/// should be given as the first parameter.
///
/// The passed in callback is where the actual test body should go. Setup and
/// cleanup happens automatically.
pub fn run_syncback_test(name: &str, callback: impl FnOnce(&Path)) {
    let _ = env_logger::try_init();

    // let working_dir = get_working_dir_path();

    let source_path = Path::new(SYNCBACK_TESTS_PATH)
        .join(name)
        .join(INPUT_FILE_PROJECT);
    // We want to support both rbxls and rbxms as input
    let input_file = {
        let mut path = Path::new(SYNCBACK_TESTS_PATH)
            .join(name)
            .join(INPUT_FILE_PLACE);
        if !path.exists() {
            path.set_file_name(INPUT_FILE_MODEL);
        }
        path
    };

    let test_dir = tempdir().expect("Couldn't create temporary directory");
    let project_path = test_dir
        .path()
        .canonicalize()
        .expect("Couldn't canonicalize temporary directory path")
        .join(name);

    let source_is_file = fs_err::metadata(&source_path).unwrap().is_file();

    if source_is_file {
        fs_err::copy(&source_path, &project_path).expect("couldn't copy project file");
    } else {
        fs_err::create_dir(&project_path).expect("Couldn't create temporary project subdirectory");

        copy_recursive(&source_path, &project_path)
            .expect("Couldn't copy project to temporary directory");
    };

    let output = Command::new(ROJO_PATH)
        // I don't really understand why setting the working directory breaks this, but it does.
        // It's a bit concerning but I'm more interested in writing tests than debugging it right now.
        // TODO: Figure out why and fix it.
        // .current_dir(working_dir)
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

    if !output.status.success() {
        let mut lock = std::io::stderr().lock();
        writeln!(
            lock,
            "Rojo exited with status code {:?}",
            output.status.code()
        )
        .unwrap();
        writeln!(lock, "Stdout from process:").unwrap();
        lock.write_all(&output.stdout).unwrap();
        writeln!(lock, "Stderr from process:").unwrap();
        lock.write_all(&output.stderr).unwrap();

        std::process::exit(1)
    }

    let mut settings = insta::Settings::new();
    let snapshot_path = Path::new(SYNCBACK_TESTS_PATH)
        .parent()
        .unwrap()
        .join("syncback-test-snapshots");
    settings.set_snapshot_path(snapshot_path);
    settings.set_sort_maps(true);

    settings.bind(|| {
        assert_snapshot!(
            format!("{name}-stdout"),
            String::from_utf8_lossy(&output.stdout)
        )
    });

    settings.bind(|| callback(project_path.as_path()))
}

pub fn snapshot_rbxm(name: &str, input: Vec<u8>, file_name: &str) {
    assert_yaml_snapshot!(
        name,
        rbx_binary::text_format::DecodedModel::from_reader(input.as_slice()),
        file_name
    )
}
