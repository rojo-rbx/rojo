use std::{fs, path::Path, process::Command};

use insta::assert_snapshot;
use tempfile::tempdir;

use crate::rojo_test::io_util::{get_working_dir_path, BUILD_TESTS_PATH, ROJO_PATH};

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
    init_csv_with_children,
    attributes,
    client_in_folder,
    client_init,
    csv_bug_145,
    csv_bug_147,
    csv_in_folder,
    deep_nesting,
    gitkeep,
    ignore_glob_inner,
    ignore_glob_nested,
    ignore_glob_spec,
    infer_service_name,
    infer_starter_player,
    init_meta_class_name,
    init_meta_properties,
    init_with_children,
    issue_546,
    json_as_lua,
    json_model_in_folder,
    json_model_legacy_name,
    module_in_folder,
    module_init,
    nested_runcontext,
    optional,
    project_composed_default,
    project_composed_file,
    project_root_name,
    rbxm_in_folder,
    rbxmx_in_folder,
    rbxmx_ref,
    script_meta_disabled,
    server_in_folder,
    server_init,
    txt,
    txt_in_folder,
    unresolved_values,
    weldconstraint,
    sync_rule_alone,
    sync_rule_complex,
    sync_rule_nested_projects,
    no_name_default_project,
    no_name_project,
    no_name_top_level_project,
    plugin_init,
}

fn run_build_test(test_name: &str) {
    let working_dir = get_working_dir_path();

    let input_path = Path::new(BUILD_TESTS_PATH).join(test_name);

    let output_dir = tempdir().expect("couldn't create temporary directory");
    let output_path = output_dir.path().join(format!("{}.rbxmx", test_name));

    let output = Command::new(ROJO_PATH)
        .args([
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

#[cfg(target_os = "linux")]
#[test]
fn build_without_watch_does_not_create_inotify_instance() -> Result<(), Box<dyn std::error::Error>>
{
    const CHILD_ENV: &str = "ROJO_TEST_BUILD_WITHOUT_WATCH_CHILD";

    // Isolate descriptor inspection from other tests that create file watchers.
    if std::env::var_os(CHILD_ENV).is_none() {
        let output = Command::new(std::env::current_exe()?)
            .args([
                "--exact",
                "tests::build::build_without_watch_does_not_create_inotify_instance",
                "--nocapture",
            ])
            .env(CHILD_ENV, "1")
            .output()?;
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return Ok(());
    }

    let project_dir = tempdir()?;
    fs::write(
        project_dir.path().join("default.project.json"),
        r#"{"name":"WithoutWatcher","tree":{"$className":"Folder","$path":"src"}}"#,
    )?;
    fs::create_dir(project_dir.path().join("src"))?;
    fs::write(project_dir.path().join("src/main.luau"), "return 42")?;
    let output_path = project_dir.path().join("output.rbxmx");

    librojo::cli::BuildCommand {
        project: project_dir.path().to_path_buf(),
        output: Some(output_path.clone()),
        plugin: None,
        watch: false,
    }
    .run()?;

    assert!(fs::read_to_string(output_path)?.contains("return 42"));
    // BuildCommand keeps its session alive until process exit, so an eagerly
    // allocated watcher remains observable after the build completes.
    for entry in fs::read_dir("/proc/self/fd")? {
        match fs::read_link(entry?.path()) {
            Ok(target) => assert_ne!(target, Path::new("anon_inode:inotify")),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(err.into()),
        }
    }
    Ok(())
}
