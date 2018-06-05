extern crate rouille;
extern crate serde_json;
extern crate serde;

extern crate librojo;

use std::collections::HashMap;
use std::path::PathBuf;

use rouille::Request;

use librojo::{
    session::Session,
    project::Project,
    web::{Server, WebConfig, ServerInfoResponse},
};

fn get(server: &Server, url: &str) -> String {
    let info_request = Request::fake_http("GET", url, vec![], vec![]);
    let response = server.handle_request(&info_request);

    assert_eq!(response.status_code, 200);

    let (mut reader, _) = response.data.into_reader_and_size();
    let mut body = String::new();
    reader.read_to_string(&mut body).unwrap();

    body
}

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
        let body = get(&server, "/api/rojo");
        let response = serde_json::from_str::<ServerInfoResponse>(&body).unwrap();

        assert_eq!(response.server_id, "0");
        assert_eq!(response.protocol_version, 2);
        assert_eq!(response.partitions, HashMap::new());
    }
}