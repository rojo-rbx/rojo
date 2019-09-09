use std::{collections::HashMap, path::PathBuf, sync::Arc};

use failure::Fail;
use rbx_dom_weak::RbxInstanceProperties;

use crate::{
    imfs::new::{Imfs, RealFetcher, WatchMode},
    project::{Project, ProjectLoadError},
    serve_session::ServeSession,
    snapshot::{apply_patch_set, compute_patch_set, InstancePropertiesWithMeta, RojoTree},
    snapshot_middleware::snapshot_from_imfs,
    web::LiveServer,
};

const DEFAULT_PORT: u16 = 34872;

#[derive(Debug)]
pub struct ServeOptions {
    pub fuzzy_project_path: PathBuf,
    pub port: Option<u16>,
}

#[derive(Debug, Fail)]
pub enum ServeError {
    #[fail(display = "Couldn't load project: {}", _0)]
    ProjectLoad(#[fail(cause)] ProjectLoadError),
}

impl_from!(ServeError {
    ProjectLoadError => ProjectLoad,
});

pub fn serve(options: &ServeOptions) -> Result<(), ServeError> {
    let maybe_project = match Project::load_fuzzy(&options.fuzzy_project_path) {
        Ok(project) => Some(project),
        Err(ProjectLoadError::NotFound) => None,
        Err(other) => return Err(other.into()),
    };

    let port = options
        .port
        .or_else(|| {
            maybe_project
                .as_ref()
                .and_then(|project| project.serve_port)
        })
        .unwrap_or(DEFAULT_PORT);

    println!("Rojo server listening on port {}", port);

    let mut tree = RojoTree::new(InstancePropertiesWithMeta {
        properties: RbxInstanceProperties {
            name: "ROOT".to_owned(),
            class_name: "Folder".to_owned(),
            properties: HashMap::new(),
        },
        metadata: Default::default(),
    });
    let root_id = tree.get_root_id();

    let mut imfs = Imfs::new(RealFetcher::new(WatchMode::Enabled));
    let entry = imfs
        .get(&options.fuzzy_project_path)
        .expect("could not get project path");

    let snapshot = snapshot_from_imfs(&mut imfs, &entry)
        .expect("snapshot failed")
        .expect("snapshot did not return an instance");

    let patch_set = compute_patch_set(&snapshot, &tree, root_id);
    apply_patch_set(&mut tree, &patch_set);

    let session = Arc::new(ServeSession::new(maybe_project));
    let server = LiveServer::new(session);

    server.start(port);

    // let receiver = imfs.change_receiver();

    // while let Ok(change) = receiver.recv() {
    //     imfs.commit_change(&change)
    //         .expect("Failed to commit Imfs change");

    //     use notify::DebouncedEvent;
    //     if let DebouncedEvent::Write(path) = change {
    //         let contents = imfs.get_contents(path)
    //             .expect("Failed to read changed path");

    //         println!("{:?}", std::str::from_utf8(contents));
    //     }
    // }

    Ok(())
}
