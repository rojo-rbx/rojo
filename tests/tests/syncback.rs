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
                        snapshot_rbxm(&snapshot_name, content);
                    } else {
                        let content = fs::read_to_string(new).unwrap();
                        assert_snapshot!(snapshot_name, content);
                    }
                }
            });
        }
    )*};
}

syncback_tests! {
    string_value_project => ["default.project.json"],
    unscriptable_properties => ["default.project.json"],
}
