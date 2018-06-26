use std::path::PathBuf;
use std::process;
use std::fs;

use rand;

use project::Project;
use web::{self, WebConfig};
use session::Session;
use roblox_studio;

pub fn serve(project_dir: &PathBuf, override_port: Option<u64>) {
    let server_id = rand::random::<u64>();

    let project = match Project::load(project_dir) {
        Ok(v) => {
            println!("Using project from {}", fs::canonicalize(project_dir).unwrap().display());
            v
        },
        Err(err) => {
            eprintln!("{}", err);
            process::exit(1);
        },
    };

    let port = override_port.unwrap_or(project.serve_port);

    println!("Using project {:#?}", project);

    roblox_studio::install_bundled_plugin().unwrap();

    let mut session = Session::new(project.clone());
    session.start();

    let web_config = WebConfig::from_session(server_id, port, &session);

    println!("Server listening on port {}", port);

    web::start(web_config);
}
