use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use librojo::{
    live_session::LiveSession,
    project::Project,
};

lazy_static::lazy_static! {
    static ref TEST_PROJECTS_ROOT: PathBuf = {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-projects")
    };
}

#[test]
fn bad_json_model() {
    let project = Project::load_fuzzy(&TEST_PROJECTS_ROOT.join("bad_json_model"))
        .expect("Project file didn't load");

    if LiveSession::new(Arc::new(project)).is_ok() {
        panic!("Project should not have succeeded");
    }
}

#[test]
fn bad_meta_lua_classname() {
    let project = Project::load_fuzzy(&TEST_PROJECTS_ROOT.join("bad_meta_lua_classname"))
        .expect("Project file didn't load");

    if LiveSession::new(Arc::new(project)).is_ok() {
        panic!("Project should not have succeeded");
    }
}

#[test]
fn bad_meta_rbxmx_properties() {
    let project = Project::load_fuzzy(&TEST_PROJECTS_ROOT.join("bad_meta_rbxmx_properties"))
        .expect("Project file didn't load");

    if LiveSession::new(Arc::new(project)).is_ok() {
        panic!("Project should not have succeeded");
    }
}

#[test]
fn bad_missing_files() {
    let project = Project::load_fuzzy(&TEST_PROJECTS_ROOT.join("bad_missing_files"))
        .expect("Project file didn't load");

    if LiveSession::new(Arc::new(project)).is_ok() {
        panic!("Project should not have succeeded");
    }
}