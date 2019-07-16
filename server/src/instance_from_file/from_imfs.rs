use std::{
    borrow::Cow,
    collections::HashMap,
    path::Path,
};

use crate::{
    imfs::{Imfs, ImfsItem, ImfsFile, ImfsDirectory},
    instance_snapshot::InstanceSnapshot,
};

use super::{
    context::{SnapshotContext},
    error::{SnapshotResult, SnapshotError, SnapshotErrorDetail},
};

const INIT_MODULE_NAME: &str = "init.lua";
const INIT_SERVER_NAME: &str = "init.server.lua";
const INIT_CLIENT_NAME: &str = "init.client.lua";

pub fn snapshot_imfs_path<'source>(
    context: &SnapshotContext,
    imfs: &'source Imfs,
    path: &Path,
    instance_name: Option<Cow<'source, str>>,
) -> SnapshotResult<'source> {
    // If the given path doesn't exist in the in-memory filesystem, we consider
    // that an error.
    match imfs.get(path) {
        Some(imfs_item) => snapshot_imfs_item(context, imfs, imfs_item, instance_name),
        None => return Err(SnapshotError::new(SnapshotErrorDetail::FileDidNotExist, Some(path))),
    }
}

fn snapshot_imfs_item<'source>(
    context: &SnapshotContext,
    imfs: &'source Imfs,
    item: &'source ImfsItem,
    instance_name: Option<Cow<'source, str>>,
) -> SnapshotResult<'source> {
    match item {
        ImfsItem::File(file) => snapshot_imfs_file(context, imfs, file, instance_name),
        ImfsItem::Directory(directory) => snapshot_imfs_directory(context, imfs, directory, instance_name),
    }
}

fn snapshot_imfs_directory<'source>(
    context: &SnapshotContext,
    imfs: &'source Imfs,
    directory: &'source ImfsDirectory,
    instance_name: Option<Cow<'source, str>>,
) -> SnapshotResult<'source> {
    let init_path = directory.path.join(INIT_MODULE_NAME);
    let init_server_path = directory.path.join(INIT_SERVER_NAME);
    let init_client_path = directory.path.join(INIT_CLIENT_NAME);

    let snapshot_name = instance_name
        .unwrap_or_else(|| {
            Cow::Borrowed(directory.path
                .file_name().expect("Could not extract file name")
                .to_str().expect("Could not convert path to UTF-8"))
        });

    let mut snapshot = if directory.children.contains(&init_path) {
        snapshot_imfs_path(context, imfs, &init_path, Some(snapshot_name))?.unwrap()
    } else if directory.children.contains(&init_server_path) {
        snapshot_imfs_path(context, imfs, &init_server_path, Some(snapshot_name))?.unwrap()
    } else if directory.children.contains(&init_client_path) {
        snapshot_imfs_path(context, imfs, &init_client_path, Some(snapshot_name))?.unwrap()
    } else {
        InstanceSnapshot {
            snapshot_id: None,
            class_name: Cow::Borrowed("Folder"),
            name: snapshot_name,
            properties: HashMap::new(),
            children: Vec::new(),
            // metadata: MetadataPerInstance {
            //     source_path: None,
            //     ignore_unknown_instances: false,
            //     project_definition: None,
            // },
        }
    };

    // if let Some(meta) = ExtraMetadata::locate(&imfs, &directory.path.join("init"))? {
    //     meta.apply(&mut snapshot)?;
    // }

    // snapshot.metadata.source_path = Some(directory.path.to_owned());

    for child_path in &directory.children {
        let child_name = child_path
            .file_name().expect("Couldn't extract file name")
            .to_str().expect("Couldn't convert file name to UTF-8");

        if child_name.ends_with(".meta.json") {
            // meta.json files don't turn into instances themselves, they just
            // modify other instances.
            continue;
        }

        match child_name {
            INIT_MODULE_NAME | INIT_SERVER_NAME | INIT_CLIENT_NAME => {
                // The existence of files with these names modifies the
                // parent instance and is handled above, so we can skip
                // them here.
                continue;
            }
            _ => {}
        }

        if let Some(child) = snapshot_imfs_path(context, imfs, child_path, None)? {
            snapshot.children.push(child);
        }
    }

    Ok(Some(snapshot))
}

fn snapshot_imfs_file<'source>(
    context: &SnapshotContext,
    imfs: &'source Imfs,
    file: &'source ImfsFile,
    instance_name: Option<Cow<'source, str>>,
) -> SnapshotResult<'source> {
    let extension = file.path.extension()
        .map(|v| v.to_str().expect("Could not convert extension to UTF-8"));

    let mut maybe_snapshot: Option<InstanceSnapshot<'source>> = match extension {
        // Some("lua") => snapshot_lua_file(file, imfs)?,
        // Some("csv") => snapshot_csv_file(file, imfs)?,
        // Some("txt") => snapshot_txt_file(file, imfs)?,
        // Some("rbxmx") => snapshot_xml_model_file(file, imfs)?,
        // Some("rbxm") => snapshot_binary_model_file(file, imfs)?,
        // Some("json") => {
        //     let file_stem = file.path
        //         .file_stem().expect("Could not extract file stem")
        //         .to_str().expect("Could not convert path to UTF-8");

        //     if file_stem.ends_with(".model") {
        //         snapshot_json_model_file(file)?
        //     } else {
        //         None
        //     }
        // },
        Some(_) | None => None,
    };

    if let Some(mut snapshot) = maybe_snapshot.as_mut() {
        // Carefully preserve name from project manifest if present.
        if let Some(snapshot_name) = instance_name {
            snapshot.name = snapshot_name;
        }
    } else {
        // info!("File generated no snapshot: {}", file.path.display());
    }

    Ok(maybe_snapshot)
}