use std::path::PathBuf;
use std::process;
use std::time::Instant;

use rand;

use project::{Project, ProjectLoadError};
use web;

pub fn serve(project_path: &PathBuf, verbose: bool, port: Option<u64>) {
    let server_id = rand::random::<u64>();

    if verbose {
        println!("Attempting to locate project at {}...", project_path.display());
    }

    let project = match Project::load(project_path) {
        Ok(v) => {
            println!("Using project from {}", project_path.display());
            v
        },
        Err(err) => {
            match err {
                ProjectLoadError::InvalidJson(serde_err) => {
                    eprintln!(
                        "Found invalid JSON!\nProject in: {}\nError: {}",
                        project_path.display(),
                        serde_err,
                    );

                    process::exit(1);
                },
                ProjectLoadError::FailedToOpen | ProjectLoadError::FailedToRead => {
                    eprintln!("Found project file, but failed to read it!");
                    eprintln!("Check the permissions of the project file in folder {}", project_path.display());

                    process::exit(1);
                },
                _ => {
                    eprintln!("Unable to find project file in folder {}", project_path.display());
                    eprintln!("Use 'rojo init' to create a new project file.");

                    process::exit(1);
                },
            }
        },
    };

    let web_config = web::WebConfig {
        verbose,
        port: port.unwrap_or(project.serve_port),
        server_id,
    };

    println!("Server listening on port {}", web_config.port);

    web::start(web_config, project.clone(), Instant::now());
}
