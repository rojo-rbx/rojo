#[macro_use] extern crate lazy_static;

extern crate librojo;

use std::{
    path::{Path, PathBuf},
};

use librojo::{
    project::Project,
};

lazy_static! {
    static ref TEST_PROJECTS_ROOT: PathBuf = {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-projects")
    };
}

#[test]
fn foo() {
    let project_file_location = TEST_PROJECTS_ROOT.join("foo.json");
    let project = Project::load_exact(&project_file_location).unwrap();

    assert_eq!(project.name, "foo");
    assert_eq!(project.tree.len(), 1);
}

#[test]
fn empty() {
    let project_file_location = TEST_PROJECTS_ROOT.join("empty/roblox-project.json");
    let project = Project::load_exact(&project_file_location).unwrap();

    assert_eq!(project.name, "empty");
    assert_eq!(project.tree.len(), 0);
}