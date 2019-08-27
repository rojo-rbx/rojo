use std::{fs, path::Path, process::Command};

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
    client_init,
    csv_bug_145,
    csv_bug_147,
    csv_in_folder,
    gitkeep,
    json_model_in_folder,
    json_model_legacy_name,
    module_in_folder,
    module_init,
    plain_gitkeep,
    rbxm_in_folder,
    rbxmx_in_folder,
    server_in_folder,
    server_init,
    txt,
    txt_in_folder,
}

#[test]
fn build_plain_txt() {
    run_build_test("plain.txt");
}

#[test]
fn build_rbxmx_ref() {
    run_build_test("rbxmx_ref.rbxmx");
}

fn run_build_test(test_name: &str) {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let build_test_path = manifest_dir.join("build-tests");
    let working_dir = manifest_dir.parent().unwrap();

    let output_dir = tempdir().expect("couldn't create temporary directory");

    let input_path = build_test_path.join(test_name);
    let output_path = output_dir.path().join(format!("{}.rbxmx", test_name));

    let mut exe_path = working_dir.join("target/debug/rojo");
    if cfg!(windows) {
        exe_path.set_extension("exe");
    }

    let status = Command::new(exe_path)
        .args(&[
            "build",
            input_path.to_str().unwrap(),
            "-o",
            output_path.to_str().unwrap(),
        ])
        .env("RUST_LOG", "error")
        .current_dir(working_dir)
        .status()
        .expect("Couldn't start Rojo");

    assert!(status.success(), "Rojo did not exit successfully");

    let contents = fs::read_to_string(&output_path).expect("Couldn't read output file");

    assert_snapshot_matches!(test_name, contents);
}
