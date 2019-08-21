use std::{
    collections::HashMap,
    path::PathBuf,
};

use rbx_dom_weak::{RbxTree, RbxInstanceProperties};
use failure::Fail;

use crate::{
    imfs::new::{Imfs, RealFetcher},
    snapshot::{apply_patch_set, compute_patch_set},
    snapshot_middleware::snapshot_from_imfs,
};

const DEFAULT_PORT: u16 = 34872;

#[derive(Debug)]
pub struct ServeOptions {
    pub fuzzy_project_path: PathBuf,
    pub port: Option<u16>,
}

#[derive(Debug, Fail)]
pub enum ServeError {
    #[fail(display = "This error cannot happen.")]
    CannotHappen,
}

pub fn serve(options: &ServeOptions) -> Result<(), ServeError> {
    // TODO: Pull port from project iff it exists.

    let port = options.port
        // .or(project.serve_port)
        .unwrap_or(DEFAULT_PORT);

    println!("Rojo server listening on port {}", port);

    let mut tree = RbxTree::new(RbxInstanceProperties {
        name: "ROOT".to_owned(),
        class_name: "Folder".to_owned(),
        properties: HashMap::new(),
    });
    let root_id = tree.get_root_id();

    let mut imfs = Imfs::new(RealFetcher::new());
    let entry = imfs.get(&options.fuzzy_project_path)
        .expect("could not get project path");

    let snapshot = snapshot_from_imfs(&mut imfs, &entry)
        .expect("snapshot failed")
        .expect("snapshot did not return an instance");

    let patch_set = compute_patch_set(&snapshot, &tree, root_id);
    apply_patch_set(&mut tree, &patch_set);

    let receiver = imfs.change_receiver();

    while let Ok(change) = receiver.recv() {
        imfs.commit_change(&change)
            .expect("Failed to commit Imfs change");

        use notify::DebouncedEvent;
        if let DebouncedEvent::Write(path) = change {
            let contents = imfs.get_contents(path)
                .expect("Failed to read changed path");

            println!("{:?}", std::str::from_utf8(contents));
        }
    }

    Ok(())
}