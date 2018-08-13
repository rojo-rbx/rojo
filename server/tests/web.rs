#[macro_use] extern crate lazy_static;

extern crate rouille;
extern crate serde_json;
extern crate serde;
extern crate tempfile;
extern crate walkdir;

extern crate librojo;

mod test_util;
use test_util::*;

use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::{File, remove_file};
use std::io::Write;
use std::path::PathBuf;

use librojo::{
    session::Session,
    project::Project,
    web::{Server, WebConfig, ServerInfoResponse, ReadResponse, ReadAllResponse, SubscribeResponse},
    rbx::RbxValue,
};

lazy_static! {
    static ref TEST_PROJECTS_ROOT: PathBuf = {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("../test-projects");
        path
    };
}

#[test]
fn empty() {
    let original_project_path = TEST_PROJECTS_ROOT.join("empty");

    let project_tempdir = tempfile::tempdir().unwrap();
    let project_path = project_tempdir.path();

    copy_recursive(&original_project_path, &project_path).unwrap();

    let project = Project::load(&project_path).unwrap();
    let mut session = Session::new(project.clone());
    session.start();

    let web_config = WebConfig::from_session(0, project.serve_port, &session);
    let server = Server::new(web_config);

    {
        let body = server.get_string("/api/rojo");
        let response = serde_json::from_str::<ServerInfoResponse>(&body).unwrap();

        assert_eq!(response.server_id, "0");
        assert_eq!(response.protocol_version, 2);
        assert_eq!(response.partitions.len(), 0);
    }

    {
        let body = server.get_string("/api/read_all");
        let response = serde_json::from_str::<ReadAllResponse>(&body).unwrap();

        assert_eq!(response.server_id, "0");
        assert_eq!(response.message_cursor, -1);
        assert_eq!(response.instances.len(), 0);
    }

    {
        let body = server.get_string("/api/read/0");
        let response = serde_json::from_str::<ReadResponse>(&body).unwrap();

        assert_eq!(response.server_id, "0");
        assert_eq!(response.message_cursor, -1);
        assert_eq!(response.instances.len(), 0);
    }
}

#[test]
fn one_partition() {
    let original_project_path = TEST_PROJECTS_ROOT.join("one-partition");

    let project_tempdir = tempfile::tempdir().unwrap();
    let project_path = project_tempdir.path();

    copy_recursive(&original_project_path, &project_path).unwrap();

    let project = Project::load(&project_path).unwrap();
    let mut session = Session::new(project.clone());
    session.start();

    let web_config = WebConfig::from_session(0, project.serve_port, &session);
    let server = Server::new(web_config);

    {
        let body = server.get_string("/api/rojo");
        let response = serde_json::from_str::<ServerInfoResponse>(&body).unwrap();

        let mut partitions = HashMap::new();
        partitions.insert("lib".to_string(), vec!["ReplicatedStorage".to_string(), "OnePartition".to_string()]);

        assert_eq!(response.server_id, "0");
        assert_eq!(response.protocol_version, 2);
        assert_eq!(response.partitions, partitions);
    }

    let initial_body = server.get_string("/api/read_all");
    let initial_response = {
        let response = serde_json::from_str::<ReadAllResponse>(&initial_body).unwrap();

        assert_eq!(response.server_id, "0");
        assert_eq!(response.message_cursor, -1);
        assert_eq!(response.instances.len(), 4);

        let partition_id = *response.partition_instances.get("lib").unwrap();

        let mut root_id = None;
        let mut module_id = None;
        let mut client_id = None;
        let mut server_id = None;

        for (id, instance) in response.instances.iter() {
            match (instance.name.as_str(), instance.class_name.as_str()) {
                // TOOD: Should partition roots (and other directories) be some
                // magical object instead of Folder?
                // TODO: Should this name actually equal the last part of the
                // partition's target?
                ("OnePartition", "Folder") => {
                    assert!(root_id.is_none());
                    root_id = Some(*id);

                    assert_eq!(*id, partition_id);

                    assert_eq!(instance.properties.len(), 0);
                    assert_eq!(instance.parent, None);
                    assert_eq!(instance.children.len(), 3);

                    let single_body = server.get_string(&format!("/api/read/{}", id));
                    let single_response = serde_json::from_str::<ReadResponse>(&single_body).unwrap();

                    let single_instance = single_response.instances.get(id).unwrap();

                    assert_eq!(single_instance, &Cow::Borrowed(instance));
                },
                ("a", "ModuleScript") => {
                    assert!(module_id.is_none());
                    module_id = Some(*id);

                    let mut properties = HashMap::new();
                    properties.insert("Source".to_string(), RbxValue::String { value: "-- a.lua".to_string() });

                    assert_eq!(instance.properties, properties);
                    assert_eq!(instance.parent, Some(partition_id));
                    assert_eq!(instance.children.len(), 0);

                    let single_body = server.get_string(&format!("/api/read/{}", id));
                    let single_response = serde_json::from_str::<ReadResponse>(&single_body).unwrap();

                    let single_instance = single_response.instances.get(id).unwrap();

                    assert_eq!(single_instance, &Cow::Borrowed(instance));
                },
                ("b", "LocalScript") => {
                    assert!(client_id.is_none());
                    client_id = Some(*id);

                    let mut properties = HashMap::new();
                    properties.insert("Source".to_string(), RbxValue::String { value: "-- b.client.lua".to_string() });

                    assert_eq!(instance.properties, properties);
                    assert_eq!(instance.parent, Some(partition_id));
                    assert_eq!(instance.children.len(), 0);

                    let single_body = server.get_string(&format!("/api/read/{}", id));
                    let single_response = serde_json::from_str::<ReadResponse>(&single_body).unwrap();

                    let single_instance = single_response.instances.get(id).unwrap();

                    assert_eq!(single_instance, &Cow::Borrowed(instance));
                },
                ("a", "Script") => {
                    assert!(server_id.is_none());
                    server_id = Some(*id);

                    let mut properties = HashMap::new();
                    properties.insert("Source".to_string(), RbxValue::String { value: "-- a.server.lua".to_string() });

                    assert_eq!(instance.properties, properties);
                    assert_eq!(instance.parent, Some(partition_id));
                    assert_eq!(instance.children.len(), 0);

                    let single_body = server.get_string(&format!("/api/read/{}", id));
                    let single_response = serde_json::from_str::<ReadResponse>(&single_body).unwrap();

                    let single_instance = single_response.instances.get(id).unwrap();

                    assert_eq!(single_instance, &Cow::Borrowed(instance));
                },
                _ => {
                    panic!("Unexpected instance named {} of class {}", instance.name, instance.class_name);
                },
            }
        }

        let root_id = root_id.unwrap();
        let module_id = module_id.unwrap();
        let client_id = client_id.unwrap();
        let server_id = server_id.unwrap();

        {
            let root_instance = response.instances.get(&root_id).unwrap();

            assert!(root_instance.children.contains(&module_id));
            assert!(root_instance.children.contains(&client_id));
            assert!(root_instance.children.contains(&server_id));
        }

        response
    };

    {
        let temp_name = project_path.join("lib/c.client.lua");

        {
            let mut file = File::create(&temp_name).unwrap();
            file.write_all(b"-- c.client.lua").unwrap();
        }

        {
            // Block until Rojo detects the addition of our temp file
            let body = server.get_string("/api/subscribe/-1");
            let response = serde_json::from_str::<SubscribeResponse>(&body).unwrap();

            assert_eq!(response.server_id, "0");
            assert_eq!(response.message_cursor, 0);
            assert_eq!(response.messages.len(), 1);

            // TODO: Read which instance was changed and try to access it with
            // /read
        }

        let body = server.get_string("/api/read_all");
        let response = serde_json::from_str::<ReadAllResponse>(&body).unwrap();

        assert_eq!(response.server_id, "0");
        assert_eq!(response.message_cursor, 0);
        assert_eq!(response.instances.len(), 5);

        let partition_id = *response.partition_instances.get("lib").unwrap();

        let mut root_id = None;
        let mut module_id = None;
        let mut client_id = None;
        let mut server_id = None;
        let mut new_id = None;

        for (id, instance) in response.instances.iter() {
            match (instance.name.as_str(), instance.class_name.as_str()) {
                // TOOD: Should partition roots (and other directories) be some
                // magical object instead of Folder?
                // TODO: Should this name actually equal the last part of the
                // partition's target?
                ("OnePartition", "Folder") => {
                    assert!(root_id.is_none());
                    root_id = Some(*id);

                    assert_eq!(*id, partition_id);

                    assert_eq!(instance.properties.len(), 0);
                    assert_eq!(instance.parent, None);
                    assert_eq!(instance.children.len(), 4);

                    let single_body = server.get_string(&format!("/api/read/{}", id));
                    let single_response = serde_json::from_str::<ReadResponse>(&single_body).unwrap();

                    let single_instance = single_response.instances.get(id).unwrap();

                    assert_eq!(single_instance, &Cow::Borrowed(instance));
                },
                ("a", "ModuleScript") => {
                    assert!(module_id.is_none());
                    module_id = Some(*id);

                    let mut properties = HashMap::new();
                    properties.insert("Source".to_string(), RbxValue::String { value: "-- a.lua".to_string() });

                    assert_eq!(instance.properties, properties);
                    assert_eq!(instance.parent, Some(partition_id));
                    assert_eq!(instance.children.len(), 0);

                    let single_body = server.get_string(&format!("/api/read/{}", id));
                    let single_response = serde_json::from_str::<ReadResponse>(&single_body).unwrap();

                    let single_instance = single_response.instances.get(id).unwrap();

                    assert_eq!(single_instance, &Cow::Borrowed(instance));
                },
                ("b", "LocalScript") => {
                    assert!(client_id.is_none());
                    client_id = Some(*id);

                    let mut properties = HashMap::new();
                    properties.insert("Source".to_string(), RbxValue::String { value: "-- b.client.lua".to_string() });

                    assert_eq!(instance.properties, properties);
                    assert_eq!(instance.parent, Some(partition_id));
                    assert_eq!(instance.children.len(), 0);

                    let single_body = server.get_string(&format!("/api/read/{}", id));
                    let single_response = serde_json::from_str::<ReadResponse>(&single_body).unwrap();

                    let single_instance = single_response.instances.get(id).unwrap();

                    assert_eq!(single_instance, &Cow::Borrowed(instance));
                },
                ("a", "Script") => {
                    assert!(server_id.is_none());
                    server_id = Some(*id);

                    let mut properties = HashMap::new();
                    properties.insert("Source".to_string(), RbxValue::String { value: "-- a.server.lua".to_string() });

                    assert_eq!(instance.properties, properties);
                    assert_eq!(instance.parent, Some(partition_id));
                    assert_eq!(instance.children.len(), 0);

                    let single_body = server.get_string(&format!("/api/read/{}", id));
                    let single_response = serde_json::from_str::<ReadResponse>(&single_body).unwrap();

                    let single_instance = single_response.instances.get(id).unwrap();

                    assert_eq!(single_instance, &Cow::Borrowed(instance));
                },
                ("c", "LocalScript") => {
                    assert!(new_id.is_none());
                    new_id = Some(*id);

                    let mut properties = HashMap::new();
                    properties.insert("Source".to_string(), RbxValue::String { value: "-- c.client.lua".to_string() });

                    assert_eq!(instance.properties, properties);
                    assert_eq!(instance.parent, Some(partition_id));
                    assert_eq!(instance.children.len(), 0);

                    let single_body = server.get_string(&format!("/api/read/{}", id));
                    let single_response = serde_json::from_str::<ReadResponse>(&single_body).unwrap();

                    let single_instance = single_response.instances.get(id).unwrap();

                    assert_eq!(single_instance, &Cow::Borrowed(instance));
                },
                _ => {},
            }
        }

        let root_id = root_id.unwrap();
        let module_id = module_id.unwrap();
        let client_id = client_id.unwrap();
        let server_id = server_id.unwrap();
        let new_id = new_id.unwrap();

        let root_instance = response.instances.get(&root_id).unwrap();

        assert!(root_instance.children.contains(&module_id));
        assert!(root_instance.children.contains(&client_id));
        assert!(root_instance.children.contains(&server_id));
        assert!(root_instance.children.contains(&new_id));

        remove_file(&temp_name).unwrap();
    }

    {
        // Block until Rojo detects the removal of our temp file
        let body = server.get_string("/api/subscribe/0");
        let response = serde_json::from_str::<SubscribeResponse>(&body).unwrap();

        assert_eq!(response.server_id, "0");
        assert_eq!(response.message_cursor, 1);
        assert_eq!(response.messages.len(), 1);
    }

    {
        // Everything should be back to the initial state!
        let body = server.get_string("/api/read_all");
        let response = serde_json::from_str::<ReadAllResponse>(&body).unwrap();

        assert_eq!(response.server_id, "0");
        assert_eq!(response.message_cursor, 1);
        assert_eq!(response.instances.len(), 4);

        assert_eq!(response.instances, initial_response.instances);
    }

    // TODO: Test to change existing instance
}

#[test]
fn partition_to_file() {
    let original_project_path = TEST_PROJECTS_ROOT.join("partition-to-file");

    let project_tempdir = tempfile::tempdir().unwrap();
    let project_path = project_tempdir.path();

    copy_recursive(&original_project_path, &project_path).unwrap();

    let project = Project::load(&project_path).unwrap();
    let mut session = Session::new(project.clone());
    session.start();

    let web_config = WebConfig::from_session(0, project.serve_port, &session);
    let server = Server::new(web_config);

    {
        let body = server.get_string("/api/rojo");
        let response = serde_json::from_str::<ServerInfoResponse>(&body).unwrap();

        assert_eq!(response.server_id, "0");
        assert_eq!(response.protocol_version, 2);
        assert_eq!(response.partitions.len(), 1);
    }

    {
        let body = server.get_string("/api/read_all");
        let response = serde_json::from_str::<ReadAllResponse>(&body).unwrap();

        assert_eq!(response.server_id, "0");
        assert_eq!(response.message_cursor, -1);
        assert_eq!(response.instances.len(), 1);

        let instance = response.instances.values().next().unwrap();

        assert_eq!(instance.name, "bar");
        assert_eq!(instance.class_name, "ModuleScript");
        assert_eq!(instance.properties.get("Source"), Some(&RbxValue::String { value: "-- foo.lua".to_string() }));
        assert_eq!(instance.children.len(), 0);
        assert_eq!(instance.parent, None);

        let body = server.get_string("/api/read/0");
        let response = serde_json::from_str::<ReadResponse>(&body).unwrap();

        assert_eq!(response.server_id, "0");
        assert_eq!(response.message_cursor, -1);
        assert_eq!(response.instances.len(), 1);

        let single_instance = response.instances.values().next().unwrap();

        assert_eq!(&Cow::Borrowed(instance), single_instance);
    }

    let file_path = project_path.join("foo.lua");

    {
        let mut file = File::create(file_path).unwrap();
        file.write_all(b"-- modified").unwrap();
    }

    {
        // Block until Rojo detects our file being modified
        let body = server.get_string("/api/subscribe/-1");
        let response = serde_json::from_str::<SubscribeResponse>(&body).unwrap();

        assert_eq!(response.server_id, "0");
        assert_eq!(response.message_cursor, 0);
        assert_eq!(response.messages.len(), 1);
    }

    {
        let body = server.get_string("/api/read/0");
        let response = serde_json::from_str::<ReadResponse>(&body).unwrap();

        assert_eq!(response.server_id, "0");
        assert_eq!(response.message_cursor, 0);
        assert_eq!(response.instances.len(), 1);

        let instance = response.instances.values().next().unwrap();

        assert_eq!(instance.name, "bar");
        assert_eq!(instance.class_name, "ModuleScript");
        assert_eq!(instance.properties.get("Source"), Some(&RbxValue::String { value: "-- modified".to_string() }));
        assert_eq!(instance.children.len(), 0);
        assert_eq!(instance.parent, None);
    }
}
