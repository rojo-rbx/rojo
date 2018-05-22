use std::path::{Path, PathBuf};
use std::process;
use std::sync::{Arc, Mutex};
use std::thread;

use rand;

use project::{Project, ProjectLoadError};
use plugin::{PluginChain};
use plugins::{DefaultPlugin, JsonModelPlugin, ScriptPlugin};
use vfs::{VfsSession, VfsWatcher};
use web;

pub fn serve(project_path: &PathBuf, verbose: bool, port: Option<u64>) {
    let server_id = rand::random::<u64>();

    let project = match Project::load(project_path) {
        Ok(project) => {
            println!("Using project \"{}\" from {}", project.name, project_path.display());
            project
        },
        Err(err) => {
            match err {
                ProjectLoadError::InvalidJson(serde_err) => {
                    eprintln!("Project contained invalid JSON!");
                    eprintln!("{}", project_path.display());
                    eprintln!("Error: {}", serde_err);

                    process::exit(1);
                },
                ProjectLoadError::FailedToOpen | ProjectLoadError::FailedToRead => {
                    eprintln!("Found project file, but failed to read it!");
                    eprintln!("Check the permissions of the project file at {}", project_path.display());

                    process::exit(1);
                },
                ProjectLoadError::DidNotExist => {
                    eprintln!("Found no project file! Create one using 'rojo init'");
                    eprintln!("Checked for a project at {}", project_path.display());

                    process::exit(1);
                },
            }
        },
    };

    if project.partitions.len() == 0 {
        println!("");
        println!("This project has no partitions and will not do anything when served!");
        println!("This is usually a mistake -- edit rojo.json!");
        println!("");
    }

    lazy_static! {
        static ref PLUGIN_CHAIN: PluginChain = PluginChain::new(vec![
            Box::new(ScriptPlugin::new()),
            Box::new(JsonModelPlugin::new()),
            Box::new(DefaultPlugin::new()),
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

    {
        let vfs = vfs.clone();
        thread::spawn(move || {
            VfsWatcher::new(vfs).start();
        });
    }

    let web_config = web::WebConfig {
        verbose,
        port: port.unwrap_or(project.serve_port),
        server_id,
    };

    println!("Server listening on port {}", web_config.port);

    web::start(web_config, project.clone(), &PLUGIN_CHAIN, vfs.clone());
}
