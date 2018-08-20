use std::{
    path::Path,
    process,
    sync::Arc,
};

use ::{
    project::Project,
    web::Server,
    session::Session,
    roblox_studio,
};

pub fn serve(fuzzy_project_location: &Path) {
    let project = match Project::load_fuzzy(fuzzy_project_location) {
        Ok(project) => project,
        Err(error) => {
            eprintln!("Fatal: {}", error);
            process::exit(1);
        },
    };

    println!("Found project at {}", project.file_location.display());
    println!("Using project {:#?}", project);

    roblox_studio::install_bundled_plugin().unwrap();

    let session = Arc::new(Session::new(project));
    let server = Server::new(Arc::from(session));

    println!("Server listening on port 8000.");

    server.listen(8000);
}