use std::{fs, path::Path, process::Command};

use insta::assert_snapshot;
use tempfile::tempdir;

use crate::rojo_test::io_util::{get_working_dir_path, BUILD_TESTS_PATH, ROJO_PATH};

macro_rules! gen_build_tests {
    ( $($test_name:ident: $out_format:literal,)* ) => {
        $(
            paste::item! {
                #[test]
                fn [<build_ $test_name>]() {
                    let _ = env_logger::try_init();

                    run_build_test(stringify!($test_name), $out_format);
                }
            }
        )*
    };
}

gen_build_tests! {
    init_csv_with_children: "rbxmx",
    attributes: "rbxmx",
    client_in_folder: "rbxmx",
    client_init: "rbxmx",
    csv_bug_145: "rbxmx",
    csv_bug_147: "rbxmx",
    csv_in_folder: "rbxmx",
    deep_nesting: "rbxmx",
    gitkeep: "rbxmx",
    ignore_glob_inner: "rbxmx",
    ignore_glob_nested: "rbxmx",
    ignore_glob_spec: "rbxmx",
    infer_service_name: "rbxlx",
    infer_starter_player: "rbxlx",
    init_meta_class_name: "rbxmx",
    init_meta_properties: "rbxmx",
    init_with_children: "rbxmx",
    issue_546: "rbxmx",
    json_as_lua: "rbxmx",
    json_model_in_folder: "rbxmx",
    json_model_legacy_name: "rbxmx",
    module_in_folder: "rbxmx",
    module_init: "rbxmx",
    optional: "rbxmx",
    project_composed_default: "rbxmx",
    project_composed_file: "rbxmx",
    project_root_name: "rbxmx",
    rbxm_in_folder: "rbxmx",
    rbxmx_in_folder: "rbxmx",
    rbxmx_ref: "rbxmx",
    script_meta_disabled: "rbxmx",
    server_in_folder: "rbxmx",
    server_init: "rbxmx",
    txt: "rbxmx",
    txt_in_folder: "rbxmx",
    unresolved_values: "rbxlx",
    weldconstraint: "rbxmx",
}

fn run_build_test(test_name: &str, out_format: &str) {
    let working_dir = get_working_dir_path();

    let input_path = Path::new(BUILD_TESTS_PATH).join(test_name);

    let output_dir = tempdir().expect("couldn't create temporary directory");
    let output_path = output_dir
        .path()
        .join(format!("{}.{}", test_name, out_format));

    let output = Command::new(ROJO_PATH)
        .args(&[
            "build",
            input_path.to_str().unwrap(),
            "-o",
            output_path.to_str().unwrap(),
        ])
        .env("RUST_LOG", "error")
        .current_dir(working_dir)
        .output()
        .expect("Couldn't start Rojo");

    print!("{}", String::from_utf8_lossy(&output.stdout));
    eprint!("{}", String::from_utf8_lossy(&output.stderr));

    assert!(output.status.success(), "Rojo did not exit successfully");

    let contents = fs::read_to_string(&output_path).expect("Couldn't read output file");

    let mut settings = insta::Settings::new();

    let snapshot_path = Path::new(BUILD_TESTS_PATH)
        .parent()
        .unwrap()
        .join("build-test-snapshots");

    settings.set_snapshot_path(snapshot_path);

    settings.bind(|| {
        assert_snapshot!(test_name, contents);
    });
}
