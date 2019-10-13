use std::{fs, path::Path, process::Command};

use insta::assert_snapshot;
use tempfile::tempdir;

use crate::util::{get_build_tests_path, get_rojo_path, get_working_dir_path};

macro_rules! gen_build_tests {
    ( $($test_name: ident,)* ) => {
        $(
            paste::item! {
                #[test]
                fn [<build_ $test_name>]() {
                    let _ = env_logger::try_init();

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
    deep_nesting,
    gitkeep,
    init_meta_class_name,
    init_meta_properties,
    init_with_children,
    json_model_in_folder,
    json_model_legacy_name,
    module_in_folder,
    module_init,
    plain_gitkeep,
    rbxm_in_folder,
    rbxmx_in_folder,
    script_meta_disabled,
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
    let build_test_path = get_build_tests_path();
    let working_dir = get_working_dir_path();

    let input_path = build_test_path.join(test_name);

    let output_dir = tempdir().expect("couldn't create temporary directory");
    let output_path = output_dir.path().join(format!("{}.rbxmx", test_name));

    let exe_path = get_rojo_path();

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

    let mut settings = insta::Settings::new();

    let snapshot_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("build-test-snapshots");
    settings.set_snapshot_path(snapshot_path);

    settings.bind(|| {
        assert_snapshot!(test_name, contents);
    });
}
