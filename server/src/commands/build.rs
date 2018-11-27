use std::{
    path::PathBuf,
    fs::File,
    process,
};

use rbxmx;

use crate::{
    rbx_session::construct_oneoff_tree,
    project::Project,
    imfs::Imfs,
};

#[derive(Debug)]
pub struct BuildOptions {
    pub fuzzy_project_path: PathBuf,
    pub output_file: PathBuf,
}

pub fn build(options: &BuildOptions) {
    info!("Looking for project at {}", options.fuzzy_project_path.display());

    let project = match Project::load_fuzzy(&options.fuzzy_project_path) {
        Ok(project) => project,
        Err(error) => {
            error!("{}", error);
            process::exit(1);
        },
    };

    info!("Found project at {}", project.file_location.display());
    info!("Using project {:#?}", project);

    let imfs = Imfs::new(&project)
        .expect("Could not create in-memory filesystem");

    let tree = construct_oneoff_tree(&project, &imfs);
    let root_id = tree.get_root_id();

    let mut file = File::create(&options.output_file)
        .expect("Could not open output file for write");

    rbxmx::encode(&tree, &[root_id], &mut file);
}