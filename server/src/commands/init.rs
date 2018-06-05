use std::path::PathBuf;
use std::process;

use project::SourceProject;

pub fn init(project_path: &PathBuf) {
    match SourceProject::init(project_path) {
        Ok(_) => {
            println!("Created new empty project at {}", project_path.display());
        },
        Err(e) => {
            eprintln!("Failed to create new project.\n{}", e);
            process::exit(1);
        },
    }
}
