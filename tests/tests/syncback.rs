use std::{ffi::OsStr, fs};

use insta::assert_snapshot;

use crate::rojo_test::syncback_util::{run_syncback_test, snapshot_rbxm};

macro_rules! syncback_tests {
    ($($test_name:ident => $list:expr$(,)?),*) => {$(
        #[test]
        fn $test_name() {
            run_syncback_test(stringify!($test_name), |path| {
                for name in $list {
                    let snapshot_name = format!(concat!(stringify!($test_name), "-{}"), name);
                    let new = path.join::<&str>(name);
                    if let Some("rbxm") = new.extension().and_then(OsStr::to_str) {
                        let content = fs::read(new).unwrap();
                        snapshot_rbxm(&snapshot_name, content, name);
                    } else {
                        let content = fs::read_to_string(new).unwrap();
                        assert_snapshot!(snapshot_name, content, name);
                    }
                }
            });
        }
    )*};
}

// TODO: All middleware, project all middleware
// probably replace them with tests for CSV and the like

syncback_tests! {
    // Ensures that there's only one copy written to disk if navigating a
    // project file might yield two copies
    child_but_not => ["OnlyOneCopy/child_of_one.luau", "ReplicatedStorage/child_replicated_storage.luau"],
    // Ensures that if a RojoId is duplicated somewhere in the project, it's
    // rewritten rather than synced back as a conflict
    duplicate_rojo_id => ["container.model.json"],
    // Ensures that the `ignorePaths` setting works for additions
    ignore_paths_adding => ["src/int_value.model.json", "src/subfolder/string_value.txt"],
    // Ensures that the `ignorePaths` setting works for removals
    ignore_paths_removing => ["src/Message.rbxm"],
    // Ensures that `ignoreTrees` works for additions
    ignore_trees_adding => [],
    // Ensures that `ignoreTrees` works for removals
    ignore_trees_removing => [],
    // Ensures projects that refer to other projects work as expected
    nested_projects => ["nested.project.json", "string_value.txt"],
    // Ensures files that are ignored by nested projects are picked up if
    // they're included in second project. Unusual but perfectly workable
    // pattern that syncback has to support.
    nested_projects_weird => ["src/modules/ClientModule.luau", "src/modules/ServerModule.luau"],
    // Ensures that projects respect `init` files when they're directly referenced from a node
    project_init => ["src/init.luau"],
    // Ensures that StringValues inside project files are written to the
    // project file, but only if they don't have `$path` set
    string_value_project => ["default.project.json"],
    // Ensures that the `syncUnscriptable` setting works
    unscriptable_properties => ["default.project.json"],
}
