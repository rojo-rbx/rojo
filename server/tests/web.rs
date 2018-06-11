extern crate rouille;
extern crate serde_json;
extern crate serde;

extern crate librojo;

mod test_util;
use test_util::*;

use std::collections::HashMap;
use std::path::PathBuf;
use std::borrow::Cow;

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

    let check_base_read_all = || {
        let body = server.get_string("/api/read_all");
        let response = serde_json::from_str::<ReadAllResponse>(&body).unwrap();

        let partition_id = *response.partition_instances.get("lib").unwrap();

        assert_eq!(response.server_id, "0");
        assert_eq!(response.message_cursor, -1);
        assert_eq!(response.instances.len(), 4);

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
                    properties.insert("Source".to_string(), "-- a.lua".to_string());

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
                    properties.insert("Source".to_string(), "-- b.client.lua".to_string());

                    assert_eq!(instance.name, "b");
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
                    properties.insert("Source".to_string(), "-- a.server.lua".to_string());

                    assert_eq!(instance.name, "a");
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

        let root_instance = response.instances.get(&root_id).unwrap();

        assert!(root_instance.children.contains(&module_id));
        assert!(root_instance.children.contains(&client_id));
        assert!(root_instance.children.contains(&server_id));
    };

    check_base_read_all();

    // TODO: Test /subscribe
}