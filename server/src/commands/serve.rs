use std::collections::HashMap;
use std::path::PathBuf;
use std::process;
use std::fs;

use rand;

use project::SourceProject;
use web;
use session::{Session, SessionConfig};
use partition::Partition;

pub fn serve(project_path: &PathBuf, port: Option<u64>) {
    let server_id = rand::random::<u64>();

    let project = match SourceProject::load(project_path) {
        Ok(v) => {
            println!("Using project from {}", fs::canonicalize(project_path).unwrap().display());
            v
        },
        Err(err) => {
            eprintln!("{}", err);
            process::exit(1);
        },
    };

    let mut partitions = HashMap::new();

    for (partition_name, partition) in project.partitions.iter() {
        let path = fs::canonicalize(project_path.join(&partition.path)).unwrap();
        let target = partition.target.split(".").map(String::from).collect::<Vec<_>>();

        partitions.insert(partition_name.clone(), Partition {
            path,
            target,
            name: partition_name.clone(),
        });
    }

    let config = SessionConfig {
        partitions: partitions.clone(),
    };

    println!("Using session config {:#?}", config);

    let mut session = Session::new(config.clone());
    session.start();

    let web_config = web::WebConfig {
        port: port.unwrap_or(project.serve_port),
        server_id,
        rbx_session: session.get_rbx_session(),
        message_session: session.get_message_session(),
        partitions,
    };

    println!("Server listening on port {}", web_config.port);

    web::start(web_config);
}
