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
    web::{Server, WebConfig, ServerInfoResponse, ReadAllResponse},
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
        assert_eq!(response.partitions, HashMap::new());
    }

    {
        let body = server.get_string("/api/read_all");
        let response = serde_json::from_str::<ReadAllResponse>(&body).unwrap();

        assert_eq!(response.server_id, "0");
        assert_eq!(response.message_cursor, -1);
        assert_eq!(response.instances.as_ref(), &HashMap::new());
    }
}