use std::path::{Path, PathBuf};
use std::process;
use std::sync::{Arc, Mutex};

use rand;

use project::{Project, ProjectLoadError};
use middleware::{MiddlewareChain};
use middlewares::{DefaultMiddleware, JsonModelMiddleware, ScriptMiddleware};
use vfs::{VfsSession};
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
                    eprintln!(
                        "Check the permissions of the project file at\n{}",
                        project_path.display(),
                    );

                    process::exit(1);
                },
                _ => {
                    // Any other error is fine; use the default project.
                    println!("Found no project file, using default project...");
                    Project::default()
                },
            }
        },
    };

    let web_config = web::WebConfig {
        verbose,
        port: port.unwrap_or(project.serve_port),
        server_id,
    };

    lazy_static! {
        static ref PLUGIN_CHAIN: MiddlewareChain = MiddlewareChain::new(vec![
            Box::new(ScriptMiddleware::new()),
            Box::new(JsonModelMiddleware::new()),
            Box::new(DefaultMiddleware::new()),
        ]);
    }

    let vfs = {
        let mut vfs = VfsSession::new(&PLUGIN_CHAIN);

        for (name, project_partition) in &project.partitions {
            let path = {
                let given_path = Path::new(&project_partition.path);

                if given_path.is_absolute() {
                    given_path.to_path_buf()
                } else {
                    project_path.join(given_path)
                }
            };

            vfs.insert_partition(name, path);
        }

        Arc::new(Mutex::new(vfs))
    };

    println!("Server listening on port {}", web_config.port);

    web::start(web_config, project.clone(), &PLUGIN_CHAIN, vfs.clone());
}
