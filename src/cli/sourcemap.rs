use std::{
    sync::MutexGuard,
    io::{BufWriter, Write, stdout},
    path::PathBuf
};

use memofs::Vfs;
use fs_err::File;
use serde::Serialize;
use structopt::StructOpt;

use crate::{
    serve_session::ServeSession,
    snapshot::{
        RojoTree,
        InstanceWithMeta
    }
};

use super::resolve_path;

const PATH_STRIP_FAILED_ERR: &str = "Failed to create relative paths for project file!";

/// Representation of a node in the generated sourcemap tree.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SourcemapNode {
    name: String,
    class_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    file_paths: Option<Vec<PathBuf>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    children: Option<Vec<SourcemapNode>>,
}

/// Generates a sourcemap file from the Rojo project.
#[derive(Debug, StructOpt)]
pub struct SourcemapCommand {
    /// Path to the project to use for the sourcemap. Defaults to the current directory.
    #[structopt(default_value = "")]
    pub project: PathBuf,

    /// Where to output the sourcemap. Omit this to use stdout instead of writing to a file.
    ///
    /// Should end in .json.
    #[structopt(long, short)]
    pub output: Option<PathBuf>,

    /// If non-script files should be included or not. Defaults to false.
    #[structopt(long)]
    pub include_non_scripts: bool,
}

impl SourcemapCommand {
    pub fn run(self) -> anyhow::Result<()> {
        let project_path = resolve_path(&self.project);

        let mut project_dir = project_path.to_path_buf();
        project_dir.pop();

        log::trace!("Constructing in-memory filesystem");
        let vfs = Vfs::new_default();

        let session = ServeSession::new(vfs, &project_path)?;

        let tree = session.tree();

        let root_node = recurse_create_node(
            &tree,
            &tree.get_instance(tree.get_root_id()).unwrap(),
            &project_dir,
            &self.include_non_scripts
        );

        if let Some(output_path) = self.output {
            let mut file = BufWriter::new(File::create(&output_path)?);
            serde_json::to_writer(&mut file, &root_node)?;
            file.flush()?;

            let filename = &output_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("<invalid utf-8>");
            println!("Created sourcemap at {}", filename);
        } else {
            let str = serde_json::to_string(&root_node)?;
            stdout().write(str.as_bytes())?;
        }

        Ok(())
    }
}

fn recurse_create_node(
    tree: &MutexGuard<RojoTree>,
    instance: &InstanceWithMeta,
    project_dir: &PathBuf,
    include_non_scripts: &bool
) -> Option<SourcemapNode> {

    let mut node_children = Vec::new();
    for child_id in instance.children() {
        if let Some(child_instance) = tree.get_instance(child_id.to_owned()) {
            if let Some(child_node) = recurse_create_node(
                tree,
                &child_instance,
                &project_dir,
                &include_non_scripts
            ) {
                node_children.push(child_node);
            }
        }
    }

    // If we only want to include scripts, make sure this instance is a script,
    // or that is has children, meaning it then potentiallly has script descendants
    // This works and does not create unnecessary empty non-script nodes
    // because we create children recursively *before* performing this check
    if !include_non_scripts && node_children.len() == 0 {
        let is_script = match instance.class_name() {
            "Script" | "LocalScript" | "ModuleScript" => true,
            _ => false
        };
        if !is_script {
            return None
        }
    }

    // 1. Filter out directories and paths that dont exist
    // 2. Remove the root directory (parent of project directory)
    // from the path to transform into project-relative paths
    let existing_paths = instance.metadata().relevant_paths.iter()
        .filter(|path| path.is_file() && path.exists())
        .map(|path| path.strip_prefix(project_dir).expect(PATH_STRIP_FAILED_ERR))
        .map(|path| path.to_path_buf())
        .collect::<Vec<PathBuf>>();

    Some(SourcemapNode {
        name: instance.name().to_string(),
        class_name: instance.class_name().to_string(),
        file_paths: match existing_paths.len() {
            0 => None,
            _ => Some(existing_paths)
        },
        children: match node_children.len() {
            0 => None,
            _ => Some(node_children)
        },
    })
}
