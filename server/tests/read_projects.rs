#[macro_use]
extern crate lazy_static;

use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
};

use pretty_assertions::assert_eq;
use rbx_dom_weak::RbxValue;

use librojo::project::{Project, ProjectNode};

lazy_static! {
    static ref TEST_PROJECTS_ROOT: PathBuf =
        { Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-projects") };
}

#[test]
fn empty() {
    let project_file_location = TEST_PROJECTS_ROOT.join("empty/default.project.json");
    let project = Project::load_exact(&project_file_location).unwrap();

    assert_eq!(project.name, "empty");
}

#[test]
fn empty_fuzzy_file() {
    let project_file_location = TEST_PROJECTS_ROOT.join("empty/default.project.json");
    let project = Project::load_fuzzy(&project_file_location).unwrap();

    assert_eq!(project.name, "empty");
}

#[test]
fn empty_fuzzy_folder() {
    let project_location = TEST_PROJECTS_ROOT.join("empty");
    let project = Project::load_fuzzy(&project_location).unwrap();

    assert_eq!(project.name, "empty");
}

#[test]
fn single_partition_game() {
    let project_location = TEST_PROJECTS_ROOT.join("single_partition_game");
    let project = Project::load_fuzzy(&project_location).unwrap();

    let expected_project = {
        let foo = ProjectNode {
            path: Some(project_location.join("lib")),
            ..Default::default()
        };

        let mut replicated_storage_children = BTreeMap::new();
        replicated_storage_children.insert("Foo".to_string(), foo);

        let replicated_storage = ProjectNode {
            class_name: Some(String::from("ReplicatedStorage")),
            children: replicated_storage_children,
            ..Default::default()
        };

        let mut http_service_properties = HashMap::new();
        http_service_properties.insert(
            "HttpEnabled".to_string(),
            RbxValue::Bool { value: true }.into(),
        );

        let http_service = ProjectNode {
            class_name: Some(String::from("HttpService")),
            properties: http_service_properties,
            ..Default::default()
        };

        let mut root_children = BTreeMap::new();
        root_children.insert("ReplicatedStorage".to_string(), replicated_storage);
        root_children.insert("HttpService".to_string(), http_service);

        let root_node = ProjectNode {
            class_name: Some(String::from("DataModel")),
            children: root_children,
            ..Default::default()
        };

        Project {
            name: "single-sync-point".to_string(),
            tree: root_node,
            serve_port: None,
            serve_place_ids: None,
            file_location: project_location.join("default.project.json"),
        }
    };

    assert_eq!(project, expected_project);
}

#[test]
fn single_partition_model() {
    let project_file_location = TEST_PROJECTS_ROOT.join("single_partition_model");
    let project = Project::load_fuzzy(&project_file_location).unwrap();

    assert_eq!(project.name, "test-model");
}

#[test]
fn composing_models() {
    let project_file_location = TEST_PROJECTS_ROOT.join("composing_models");
    let project = Project::load_fuzzy(&project_file_location).unwrap();

    assert_eq!(project.name, "composing-models");
}
