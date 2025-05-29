use std::fs;

use insta::assert_snapshot;

use crate::rojo_test::syncback_util::run_syncback_test;

#[test]
fn unscriptable_properties() {
    run_syncback_test("unscriptable_properties", |path| {
        let project_path = path.join("default.project.json");
        let content =
            fs::read_to_string(project_path).expect("could not read default.project.json");

        assert_snapshot!("unscriptable_properties-default.project.json", content)
    });
}
