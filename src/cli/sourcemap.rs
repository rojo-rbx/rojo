use std::{
    borrow::Cow,
    io::{BufWriter, Write},
    mem::forget,
    path::{self, Path, PathBuf},
};

use clap::Parser;
use fs_err::File;
use memofs::Vfs;
use rayon::prelude::*;
use rbx_dom_weak::{types::Ref, Ustr};
use serde::Serialize;
use tokio::runtime::Runtime;

use crate::{
    serve_session::ServeSession,
    snapshot::{AppliedPatchSet, InstanceWithMeta, RojoTree},
};

use super::resolve_path;

const PATH_STRIP_FAILED_ERR: &str = "Failed to create relative paths for project file!";
const ABSOLUTE_PATH_FAILED_ERR: &str = "Failed to turn relative path into absolute path!";

/// Representation of a node in the generated sourcemap tree.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SourcemapNode<'a> {
    name: &'a str,
    class_name: Ustr,

    #[serde(
        skip_serializing_if = "Vec::is_empty",
        serialize_with = "crate::path_serializer::serialize_vec_absolute"
    )]
    file_paths: Vec<Cow<'a, Path>>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    children: Vec<SourcemapNode<'a>>,
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

    /// Whether the sourcemap should use absolute paths instead of relative paths.
    #[clap(long)]
    pub absolute: bool,
}

impl SourcemapCommand {
    pub fn run(self) -> anyhow::Result<()> {
        let project_path = resolve_path(&self.project);

        log::trace!("Constructing in-memory filesystem");
        let vfs = Vfs::new_default();
        vfs.set_watch_enabled(self.watch);

        let session = ServeSession::new(vfs, project_path)?;
        let mut cursor = session.message_queue().cursor();

        let filter = if self.include_non_scripts {
            filter_nothing
        } else {
            filter_non_scripts
        };

        // Pre-build a rayon threadpool with a low number of threads to avoid
        // dynamic creation overhead on systems with a high number of cpus.
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus::get().min(6))
            .build_global()
            .unwrap();

        write_sourcemap(&session, self.output.as_deref(), filter, self.absolute)?;

        if self.watch {
            let rt = Runtime::new().unwrap();

            loop {
                let receiver = session.message_queue().subscribe(cursor);
                let (new_cursor, patch_set) = rt.block_on(receiver).unwrap();
                cursor = new_cursor;

                if patch_set_affects_sourcemap(&session, &patch_set, filter) {
                    write_sourcemap(&session, self.output.as_deref(), filter, self.absolute)?;
                }
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
        instance.class_name().as_str(),
        "Script" | "LocalScript" | "ModuleScript"
    )
}

fn patch_set_affects_sourcemap(
    session: &ServeSession,
    patch_set: &[AppliedPatchSet],
    filter: fn(&InstanceWithMeta) -> bool,
) -> bool {
    let tree = session.tree();

    // A sourcemap has probably changed when:
    patch_set.par_iter().any(|set| {
        // 1. An instance was removed, in which case it will no
        // longer exist in the tree and we cant check the filter
        !set.removed.is_empty()
            // 2. A newly added instance passes the filter
            || set.added.iter().any(|referent| {
                let instance = tree
                    .get_instance(*referent)
                    .expect("instance did not exist when updating sourcemap");
                filter(&instance)
            })
            // 3. An existing instance has its class name, name,
            // or file paths changed, and passes the filter
            || set.updated.iter().any(|updated| {
                let changed = updated.changed_class_name.is_some()
                    || updated.changed_name.is_some()
                    || updated.changed_metadata.is_some();
                if changed {
                    let instance = tree
                        .get_instance(updated.id)
                        .expect("instance did not exist when updating sourcemap");
                    filter(&instance)
                } else {
                    false
                }
            })
    })
}

fn recurse_create_node<'a>(
    tree: &'a RojoTree,
    referent: Ref,
    project_dir: &Path,
    filter: fn(&InstanceWithMeta) -> bool,
    use_absolute_paths: bool,
) -> Option<SourcemapNode<'a>> {
    let instance = tree.get_instance(referent).expect("instance did not exist");

    let children: Vec<_> = instance
        .children()
        .par_iter()
        .filter_map(|&child_id| {
            recurse_create_node(tree, child_id, project_dir, filter, use_absolute_paths)
        })
        .collect();

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
        .map(|path| path.as_path());

    let mut output_file_paths: Vec<Cow<'a, Path>> =
        Vec::with_capacity(instance.metadata().relevant_paths.len());

    if use_absolute_paths {
        // It's somewhat important to note here that `path::absolute` takes in a Path and returns a PathBuf
        for val in file_paths {
            output_file_paths.push(Cow::Owned(
                path::absolute(val).expect(ABSOLUTE_PATH_FAILED_ERR),
            ));
        }
    } else {
        for val in file_paths {
            output_file_paths.push(Cow::from(
                val.strip_prefix(project_dir).expect(PATH_STRIP_FAILED_ERR),
            ));
        }
    };

    Some(SourcemapNode {
        name: instance.name(),
        class_name: instance.class_name(),
        file_paths: output_file_paths,
        children,
    })
}

fn write_sourcemap(
    session: &ServeSession,
    output: Option<&Path>,
    filter: fn(&InstanceWithMeta) -> bool,
    use_absolute_paths: bool,
) -> anyhow::Result<()> {
    let tree = session.tree();

    let root_node = recurse_create_node(
        &tree,
        tree.get_root_id(),
        session.root_dir(),
        filter,
        use_absolute_paths,
    );

    if let Some(output_path) = output {
        let mut file = BufWriter::new(File::create(output_path)?);
        serde_json::to_writer(&mut file, &root_node)?;
        file.flush()?;

        println!("Created sourcemap at {}", output_path.display());
    } else {
        let output = serde_json::to_string(&root_node)?;
        println!("{}", output);
    }

    Ok(())
}
