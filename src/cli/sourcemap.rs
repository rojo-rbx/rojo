use std::{
    io::{BufWriter, Write},
    mem::forget,
    path::{Path, PathBuf},
};

use clap::Parser;
use fs_err::File;
use memofs::Vfs;
use rbx_dom_weak::types::Ref;
use serde::Serialize;
use tokio::runtime::Runtime;

use crate::{
    serve_session::ServeSession,
    snapshot::{InstanceWithMeta, RojoTree},
};

use super::resolve_path;

const PATH_STRIP_FAILED_ERR: &str = "Failed to create relative paths for project file!";

/// Representation of a node in the generated sourcemap tree.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SourcemapNode {
    name: String,
    class_name: String,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    file_paths: Vec<PathBuf>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    children: Vec<SourcemapNode>,
}

/// Generates a sourcemap file from the Rojo project.
#[derive(Debug, Parser)]
pub struct SourcemapCommand {
    /// Path to the project to use for the sourcemap. Defaults to the current
    /// directory.
    #[clap(default_value = "")]
    pub project: PathBuf,

    /// Where to output the sourcemap. Omit this to use stdout instead of
    /// writing to a file.
    ///
    /// Should end in .json.
    #[clap(long, short)]
    pub output: Option<PathBuf>,

    /// If non-script files should be included or not. Defaults to false.
    #[clap(long)]
    pub include_non_scripts: bool,

    /// Whether to automatically recreate a snapshot when any input files change.
    #[clap(long)]
    pub watch: bool,
}

impl SourcemapCommand {
    pub fn run(self) -> anyhow::Result<()> {
        let project_path = resolve_path(&self.project);

        log::trace!("Constructing in-memory filesystem");
        let vfs = Vfs::new_default();
        vfs.set_watch_enabled(self.watch);

        let session = ServeSession::new(vfs, &project_path)?;
        let mut cursor = session.message_queue().cursor();

        let filter = if self.include_non_scripts {
            filter_nothing
        } else {
            filter_non_scripts
        };

        write_sourcemap(&session, self.output.as_deref(), filter)?;

        if self.watch {
            let rt = Runtime::new().unwrap();

            loop {
                let receiver = session.message_queue().subscribe(cursor);
                let (new_cursor, _patch_set) = rt.block_on(receiver).unwrap();
                cursor = new_cursor;

                write_sourcemap(&session, self.output.as_deref(), filter)?;
            }
        }

        // Avoid dropping ServeSession: it's potentially VERY expensive to drop
        // and we're about to exit anyways.
        forget(session);

        Ok(())
    }
}

fn filter_nothing(_instance: &InstanceWithMeta) -> bool {
    true
}

fn filter_non_scripts(instance: &InstanceWithMeta) -> bool {
    matches!(
        instance.class_name(),
        "Script" | "LocalScript" | "ModuleScript"
    )
}

fn recurse_create_node(
    tree: &RojoTree,
    referent: Ref,
    project_dir: &Path,
    filter: fn(&InstanceWithMeta) -> bool,
) -> Option<SourcemapNode> {
    let instance = tree.get_instance(referent).expect("instance did not exist");

    let mut children = Vec::new();
    for &child_id in instance.children() {
        if let Some(child_node) = recurse_create_node(tree, child_id, project_dir, filter) {
            children.push(child_node);
        }
    }

    // If this object has no children and doesn't pass the filter, it doesn't
    // contain any information we're looking for.
    if children.is_empty() && !filter(&instance) {
        return None;
    }

    let file_paths = instance
        .metadata()
        .relevant_paths
        .iter()
        // Not all paths listed as relevant are guaranteed to exist.
        .filter(|path| path.is_file())
        .map(|path| path.strip_prefix(project_dir).expect(PATH_STRIP_FAILED_ERR))
        .map(|path| path.to_path_buf())
        .collect();

    Some(SourcemapNode {
        name: instance.name().to_string(),
        class_name: instance.class_name().to_string(),
        file_paths,
        children,
    })
}

fn write_sourcemap(
    session: &ServeSession,
    output: Option<&Path>,
    filter: fn(&InstanceWithMeta) -> bool,
) -> anyhow::Result<()> {
    let tree = session.tree();

    let root_node = recurse_create_node(&tree, tree.get_root_id(), session.root_dir(), filter);

    if let Some(output_path) = output {
        let mut file = BufWriter::new(File::create(&output_path)?);
        serde_json::to_writer(&mut file, &root_node)?;
        file.flush()?;

        println!("Created sourcemap at {}", output_path.display());
    } else {
        let output = serde_json::to_string(&root_node)?;
        println!("{}", output);
    }

    Ok(())
}
