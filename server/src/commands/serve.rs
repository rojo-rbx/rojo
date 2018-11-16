use std::{
    path::Path,
    process,
    sync::Arc,
};

use ::{
    project::Project,
    web::Server,
    session::Session,
    // roblox_studio,
};

pub fn serve(fuzzy_project_location: &Path) {
    info!("Looking for project at {}", fuzzy_project_location.display());

    let project = match Project::load_fuzzy(fuzzy_project_location) {
        Ok(project) => project,
        Err(error) => {
            error!("{}", error);
            process::exit(1);
        },
    };

    info!("Found project at {}", project.file_location.display());
    info!("Using project {:#?}", project);

    // roblox_studio::install_bundled_plugin().unwrap();

    let session = Arc::new(Session::new(project).unwrap());
    let server = Server::new(Arc::clone(&session));

    println!("Server listening on port 34872");

    server.listen(34872);
}