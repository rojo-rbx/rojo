#[macro_use] extern crate lazy_static;

extern crate librojo;

use std::{
    collections::HashMap,
    path::PathBuf,
};

use librojo::{
    project::Project,
};

lazy_static! {
    static ref TEST_PROJECTS_ROOT: PathBuf = {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("../test-projects");
        path
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