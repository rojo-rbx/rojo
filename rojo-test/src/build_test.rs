use std::{
    fs,
    path::Path,
    process::Command,
};

use insta::assert_snapshot_matches;
use tempfile::tempdir;

macro_rules! gen_build_tests {
    ( $($test_name: ident,)* ) => {
        $(
            paste::item! {
                #[test]
                fn [<build_ $test_name>]() {
                    run_build_test(stringify!($test_name));
                }
            }
        )*
    };
}

gen_build_tests! {
    client_in_folder,
    gitkeep,
    module_in_folder,
    rbxm_in_folder,
    rbxmx_in_folder,
    server_in_folder,
    txt_in_folder,
}

fn run_build_test(test_name: &str) {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let build_test_path = manifest_dir.join("build-tests");
    let working_dir = manifest_dir.parent().unwrap();

    let output_dir = tempdir().expect("couldn't create temporary directory");

    let input_path = build_test_path.join(test_name);
    let output_path = output_dir.path().join(format!("{}.rbxmx", test_name));

    let status = Command::new("cargo")
        .args(&[
            "run", "--quiet", "--",
            "build", input_path.to_str().unwrap(), "-o", output_path.to_str().unwrap(),
        ])
        .current_dir(working_dir)
        .status()
        .expect("Couldn't start Rojo");

    assert!(status.success(), "Rojo did not exit successfully");

    let contents = fs::read_to_string(&output_path)
        .expect("Couldn't read output file");

    assert_snapshot_matches!(test_name, contents);
}