extern crate rouille;
extern crate serde_json;
extern crate serde;

extern crate librojo;

mod test_util;
use test_util::*;

use std::collections::HashMap;
use std::path::PathBuf;

use librojo::{
    session::Session,
    project::Project,
    web::{Server, WebConfig, ServerInfoResponse, ReadResponse, ReadAllResponse},
};

#[test]
fn empty() {
    let project_path = {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("test-projects/empty");
        path
    };

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
    let project_path = {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("test-projects/one-partition");
        path
    };

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

    {
        let body = server.get_string("/api/read_all");
        let response = serde_json::from_str::<ReadAllResponse>(&body).unwrap();

        let partition_id = *response.partition_instances.get("lib").unwrap();

        assert_eq!(response.server_id, "0");
        assert_eq!(response.message_cursor, -1);
        assert_eq!(response.instances.len(), 4); // root and three children

        let mut found_root = false;
        let mut found_module = false;
        let mut found_client = false;
        let mut found_server = false;

        for (id, instance) in response.instances.iter() {
            match instance.class_name.as_str() {
                // TOOD: Should partition roots (and other directories) be some
                // magical object instead of Folder?
                "Folder" => {
                    assert!(!found_root);
                    found_root = true;

                    assert_eq!(*id, partition_id);

                    // TODO: Should this actually equal the last part of the
                    // partition's target?
                    assert_eq!(instance.name, "OnePartition");

                    assert_eq!(instance.properties.len(), 0);
                    assert_eq!(instance.parent, None);
                    assert_eq!(instance.children.len(), 3);
                },
                "ModuleScript" => {
                    assert!(!found_module);
                    found_module = true;

                    let mut properties = HashMap::new();
                    properties.insert("Source".to_string(), "-- a.lua".to_string());

                    assert_eq!(instance.name, "a");
                    assert_eq!(instance.properties, properties);
                    assert_eq!(instance.parent, Some(partition_id));
                    assert_eq!(instance.children.len(), 0);
                },
                "LocalScript" => {
                    assert!(!found_client);
                    found_client = true;

                    let mut properties = HashMap::new();
                    properties.insert("Source".to_string(), "-- b.client.lua".to_string());

                    assert_eq!(instance.name, "b");
                    assert_eq!(instance.properties, properties);
                    assert_eq!(instance.parent, Some(partition_id));
                    assert_eq!(instance.children.len(), 0);
                },
                "Script" => {
                    assert!(!found_server);
                    found_server = true;

                    let mut properties = HashMap::new();
                    properties.insert("Source".to_string(), "-- a.server.lua".to_string());

                    assert_eq!(instance.name, "a");
                    assert_eq!(instance.properties, properties);
                    assert_eq!(instance.parent, Some(partition_id));
                    assert_eq!(instance.children.len(), 0);
                },
                _ => panic!("Unexpected instance!"),
            }
        }

        assert!(found_root);
        assert!(found_module);
        assert!(found_client);
        assert!(found_server);
    }

    // TODO: Test /read
    // TODO: Test /subscribe
}