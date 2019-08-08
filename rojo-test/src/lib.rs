use std::{
    fs,
    path::Path,
    process::Command,
};

use insta::assert_snapshot_matches;
use tempfile::tempdir;

static BUILD_TESTS: &[&str] = &[
    "gitkeep",
    "txt_in_folder",
];

#[test]
fn build_tests() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let build_test_path = manifest_dir.join("build-tests");
    let working_dir = manifest_dir.parent().unwrap();

    let output_dir = tempdir().expect("couldn't create temporary directory");

    for &test_name in BUILD_TESTS {
        let input_path = build_test_path.join(test_name);
        let output_path = output_dir.path().join(format!("{}.rbxmx", test_name));

        let status = Command::new("cargo")
            .args(&[
                "run", "--",
                "build", input_path.to_str().unwrap(), "-o", output_path.to_str().unwrap(),
            ])
            .current_dir(working_dir)
            .status()
            .expect("Couldn't start Rojo");

        assert!(status.success(), "Rojo did not exit successfully");

        let contents = fs::read_to_string(&output_path)
            .expect("Couldn't read output file");

        assert_snapshot_matches!(format!("build_{}", test_name), contents);
    }
}