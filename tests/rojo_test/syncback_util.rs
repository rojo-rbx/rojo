use std::{borrow::Cow, collections::HashMap, path::Path};

use anyhow::Context;
use insta::assert_yaml_snapshot;
use librojo::{snapshot_from_vfs, syncback_loop, FsSnapshot, InstanceContext, Project, RojoTree};
use memofs::{InMemoryFs, IoResultExt, Vfs, VfsSnapshot};
use serde::Serialize;

use crate::rojo_test::io_util::{serialize_vec_absolute, SYNCBACK_TESTS_PATH};

const INPUT_FILE: &str = "input.rbxl";
const EXPECTED_DIR: &str = "expected";
const OUTPUT_DIR: &str = "output";

pub fn basic_syncback_test(name: &str) -> anyhow::Result<()> {
    let mut settings = insta::Settings::new();
    let snapshot_path = Path::new(SYNCBACK_TESTS_PATH)
        .parent()
        .unwrap()
        .join("syncback-test-snapshots");
    settings.set_snapshot_path(snapshot_path);

    let test_path = Path::new(SYNCBACK_TESTS_PATH).join(name);
    let input_path = test_path.join(INPUT_FILE);
    let expected_path = test_path.join(EXPECTED_DIR);
    let output_path = test_path.join(OUTPUT_DIR);

    let std_vfs = Vfs::new_default();
    std_vfs.set_watch_enabled(false);
    let im_vfs = {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(&output_path, to_vfs_snapshot(&std_vfs, &output_path)?)?;
        Vfs::new(imfs)
    };
    im_vfs.set_watch_enabled(false);

    let input_dom = rbx_binary::from_reader(std_vfs.read(input_path)?.as_slice())?;

    let (mut output_dom, project) =
        rojo_tree_from_path(&std_vfs, &output_path.join("default.project.json"))?;

    let fs_snapshot = syncback_loop(&std_vfs, &mut output_dom, input_dom, &project)?;

    settings
        .bind(|| assert_yaml_snapshot!(name, visualize_fs_snapshot(&fs_snapshot, &output_path)));

    fs_snapshot.write_to_vfs(&output_path, &im_vfs)?;
    let paths = fs_snapshot.added_paths();

    for path in paths {
        let trimmed = path.strip_prefix(&output_path)?;
        let expected = expected_path.join(trimmed);

        let expected_meta = std_vfs.metadata(&expected).with_not_found()?;
        let output_meta = match im_vfs.metadata(path).with_not_found()? {
            Some(meta) => meta,
            None => anyhow::bail!(
                "Somehow, a path did not exist in the InMemoryVfs: {}",
                trimmed.display()
            ),
        };

        if let Some(expected_meta) = expected_meta {
            match (expected_meta.is_dir(), output_meta.is_dir()) {
                (true, true) => {}
                (true, false) => anyhow::bail!(
                    "A path was a file when it should be a directory: {}",
                    trimmed.display()
                ),
                (false, true) => anyhow::bail!(
                    "A path was a directory when it should be a file: {}",
                    trimmed.display()
                ),
                (false, false) => {
                    let output_contents = im_vfs.read(path).unwrap();
                    let expected_contents = std_vfs.read(&expected).unwrap();

                    let normalized_output = normalize_line_endings(&output_contents);
                    let normalized_expected = normalize_line_endings(&expected_contents);
                    if normalized_output.as_slice() != normalized_expected.as_slice() {
                        let output_str = std::str::from_utf8(&normalized_output);
                        let expected_str = std::str::from_utf8(&normalized_expected);
                        let display = trimmed.display();
                        match (output_str, expected_str) {
                            (Ok(output), Ok(expected)) => anyhow::bail!(
                                "The contents of a file did not match what was expected: {display}.\n\
                                Expected: {expected}\n\
                                Actual: {output}"
                            ),
                            _ => anyhow::bail!(
                                "The contents of a file did not match what was expected: {display}. \
                                Expected {} bytes, got {}.",
                                normalized_output.len(), normalized_expected.len()
                            ),
                        }
                    }
                }
            }
        } else {
            anyhow::bail!(
                "A path existed in the output when it shouldn't: {}",
                trimmed.display()
            )
        }
    }
    Ok(())
}

fn rojo_tree_from_path(vfs: &Vfs, path: &Path) -> anyhow::Result<(RojoTree, Project)> {
    let project = Project::load_fuzzy(vfs, path)?
        .with_context(|| format!("no project file located at {}", path.display()))?;

    let context = InstanceContext::with_emit_legacy_scripts(project.emit_legacy_scripts);

    let snapshot = snapshot_from_vfs(&context, vfs, path)?.with_context(|| {
        format!(
            "could not load project at {} with snapshot middleware",
            path.display()
        )
    })?;

    Ok((RojoTree::new(snapshot), project))
}

fn to_vfs_snapshot(vfs: &Vfs, path: &Path) -> anyhow::Result<VfsSnapshot> {
    if vfs.metadata(path)?.is_dir() {
        let mut children = HashMap::new();
        for item in vfs.read_dir(path)? {
            let item = item?;
            children.insert(
                item.path().to_string_lossy().to_string(),
                to_vfs_snapshot(vfs, item.path())?,
            );
        }
        Ok(VfsSnapshot::dir(children))
    } else {
        let contents = vfs.read(path)?;
        Ok(VfsSnapshot::file(contents.as_slice()))
    }
}

/// Normalizes the line endings of a vector if it's user-readable.
/// If it isn't, the vector is returned unmodified.
fn normalize_line_endings(input: &Vec<u8>) -> Cow<Vec<u8>> {
    match std::str::from_utf8(input) {
        Ok(str) => {
            let mut new_str = Vec::with_capacity(input.len());
            for line in str.lines() {
                new_str.extend(line.as_bytes());
                new_str.push(b'\n')
            }
            new_str.pop();
            Cow::Owned(new_str)
        }
        Err(_) => Cow::Borrowed(input),
    }
}

#[derive(Default, Debug, Serialize)]
struct FsSnapshotVisual<'a> {
    #[serde(serialize_with = "serialize_vec_absolute")]
    added_files: Vec<&'a Path>,
    #[serde(serialize_with = "serialize_vec_absolute")]
    added_dirs: Vec<&'a Path>,
    #[serde(serialize_with = "serialize_vec_absolute")]
    removed_files: Vec<&'a Path>,
    #[serde(serialize_with = "serialize_vec_absolute")]
    removed_dirs: Vec<&'a Path>,
}

fn visualize_fs_snapshot<'a>(snapshot: &'a FsSnapshot, base_path: &Path) -> FsSnapshotVisual<'a> {
    let mut added_files = Vec::new();
    let mut added_dirs = Vec::new();
    let mut removed_files = Vec::new();
    let mut removed_dirs = Vec::new();

    for file in snapshot.added_files() {
        added_files.push(file.strip_prefix(base_path).unwrap())
    }
    for file in snapshot.added_dirs() {
        added_dirs.push(file.strip_prefix(base_path).unwrap())
    }
    for file in snapshot.removed_dirs() {
        removed_dirs.push(file.strip_prefix(base_path).unwrap())
    }
    for file in snapshot.removed_files() {
        removed_files.push(file.strip_prefix(base_path).unwrap())
    }

    added_files.sort_unstable();
    added_dirs.sort_unstable();
    removed_files.sort_unstable();
    removed_dirs.sort_unstable();

    // Turns out that the debug display for Path isn't stable. Who knew!
    FsSnapshotVisual {
        added_files,
        added_dirs,
        removed_files,
        removed_dirs,
    }
}
