use std::{
    path::PathBuf,
};

#[derive(Debug)]
pub struct BuildOptions {
    pub fuzzy_project_path: PathBuf,
    pub output_file: PathBuf,
}

pub fn build(options: &BuildOptions) {
    println!("build {:#?}", options);
}