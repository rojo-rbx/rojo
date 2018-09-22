use std::path::PathBuf;
use std::process;

use project::Project;

pub fn init(project_path: &PathBuf) {
    match Project::init(project_path) {
        Ok(_) => {
            println!("Created new empty project at {}", project_path.display());
        },
        Err(e) => {
            error!("Failed to create new project.\n{}", e);
            process::exit(1);
        },
    }
}