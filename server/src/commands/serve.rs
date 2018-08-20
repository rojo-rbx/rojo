use std::path::Path;
use std::process;
use std::fs;

use rand;

use project::Project;
// use web::{self, WebConfig};
// use session::Session;
use roblox_studio;

pub fn serve(fuzzy_project_location: &Path) {
    let server_id = rand::random::<u64>();

    let project = match Project::load_fuzzy(fuzzy_project_location) {
        Ok(project) => {
            println!("Using project from {}", fs::canonicalize(&project.file_location).unwrap().display());
            project
        },
        Err(error) => {
            eprintln!("{}", error);
            process::exit(1);
        },
    };

    println!("Using project {:#?}", project);

    roblox_studio::install_bundled_plugin().unwrap();

    // let mut session = Session::new(project.clone());
    // session.start();

    // let web_config = WebConfig::from_session(server_id, port, &session);

    // web::start(web_config);
}