use std::path::PathBuf;
use std::process;
use std::fs;

use rand;

use project::Project;
use web;
use session::Session;

pub fn serve(project_path: &PathBuf, port: Option<u64>) {
    let server_id = rand::random::<u64>();

    let project = match Project::load(project_path) {
        Ok(v) => {
            println!("Using project from {}", fs::canonicalize(project_path).unwrap().display());
            v
        },
        Err(err) => {
            eprintln!("{}", err);
            process::exit(1);
        },
    };

    println!("Using project {:#?}", project);

    let mut session = Session::new(project.clone());
    session.start();

    let web_config = web::WebConfig {
        port: port.unwrap_or(project.serve_port),
        server_id,
        rbx_session: session.get_rbx_session(),
        message_session: session.get_message_session(),
        partitions: project.partitions,
    };

    println!("Server listening on port {}", web_config.port);

    web::start(web_config);
}
