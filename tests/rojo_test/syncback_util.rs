use std::{
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
};

use anyhow::Context;
use insta::assert_yaml_snapshot;
use librojo::{snapshot_from_vfs, syncback_loop, FsSnapshot, InstanceContext, Project, RojoTree};
use memofs::{InMemoryFs, IoResultExt, Vfs, VfsSnapshot};
use serde::Serialize;

use crate::rojo_test::io_util::SYNCBACK_TESTS_PATH;

const OUTPUT_DIR: &str = "output";
const INPUT_FILE: &str = "input.rbxl";
const EXPECTED_DIR: &str = "expected";

pub fn basic_syncback_test(name: &str) -> anyhow::Result<()> {
    let test_path = Path::new(SYNCBACK_TESTS_PATH).join(name);
    let output_path = test_path.join(OUTPUT_DIR);
    let expected_path = test_path.join(EXPECTED_DIR);

    let mut settings = insta::Settings::new();
    let snapshot_path = Path::new(SYNCBACK_TESTS_PATH)
        .parent()
        .unwrap()
        .join("syncback-test-snapshots");
    settings.set_snapshot_path(snapshot_path);

    let std_vfs = Vfs::new_default();
    std_vfs.set_watch_enabled(false);

    let im_vfs = {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(&output_path, to_vfs_snapshot(&std_vfs, &output_path)?)?;
        Vfs::new(imfs)
    };
    im_vfs.set_watch_enabled(false);

    let input_file_path = test_path.join(INPUT_FILE);
    let input_dom = rbx_binary::from_reader(std_vfs.read(input_file_path)?.as_slice())?;

    let output_project_path = output_path.join("default.project.json");
    let (mut output_tree, output_project) = rojo_tree_from_path(&std_vfs, &output_project_path)?;
    let fs_snapshot = syncback_loop(&std_vfs, &mut output_tree, input_dom, &output_project)?;

    settings.bind(|| {
        assert_yaml_snapshot!(name, visualize_fs_snapshot(&fs_snapshot, &output_path));
    });

    // We write to the in-memory VFS and not the file system!
    fs_snapshot.write_to_vfs(&output_path, &im_vfs)?;

    // And now the hard part: diffing two sub-trees.
    let mut path_queue: VecDeque<PathBuf> = [expected_path.join("default.project.json")].into();

    while let Some(path) = path_queue.pop_front() {
        let path = path.strip_prefix(&expected_path)?;
        let path_display = path.display();

        let expected = expected_path.join(path);
        let output = output_path.join(path);

        let expected_meta = std_vfs.metadata(&expected).with_not_found()?;
        let output_meta = im_vfs.metadata(&output).with_not_found()?;

        match (expected_meta, output_meta) {
            (Some(expected_meta), Some(emitted_meta)) => {
                if expected_meta.is_dir() && emitted_meta.is_dir() {
                    for item in std_vfs.read_dir(&expected)? {
                        path_queue.push_back(item?.path().to_path_buf());
                    }
                } else if expected_meta.is_file() && emitted_meta.is_file() {
                    let expected_contents = std_vfs.read(&expected)?;
                    let output_contents = im_vfs.read(&output)?;
                    if expected_contents != output_contents {
                        anyhow::bail!("path {path_display} is not as expected.");
                    }
                } else if expected_meta.is_file() {
                    anyhow::bail!("path {path_display} should be a file but was emitted as a directory");
                } else {
                    anyhow::bail!("path {path_display} should be a directory but was emitted as a file");
                }
            }
            (Some(_), None) => anyhow::bail!(
                "path {path_display} does not exist in actual output for syncback despite being expected"
            ),
            (None, _) => anyhow::bail!("Somehow, {} did not exist on the FS.", expected.display()),
        }
    }

    Ok(())
}

fn rojo_tree_from_path(vfs: &Vfs, path: &Path) -> anyhow::Result<(RojoTree, Project)> {
    let project = Project::load_fuzzy(path)?
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

#[derive(Default, Debug, Serialize)]
struct FsSnapshotVisual<'a> {
    added_files: Vec<&'a Path>,
    added_dirs: Vec<&'a Path>,
    removed_files: Vec<&'a Path>,
    removed_dirs: Vec<&'a Path>,
}

fn visualize_fs_snapshot<'a>(snapshot: &'a FsSnapshot, base_path: &Path) -> FsSnapshotVisual<'a> {
    let map_closure = |p: &'a Path| p.strip_prefix(base_path).unwrap();

    let mut added_files: Vec<_> = snapshot
        .added_files()
        .into_iter()
        .map(map_closure)
        .collect();
    let mut added_dirs: Vec<_> = snapshot.added_dirs().into_iter().map(map_closure).collect();
    let mut removed_files: Vec<_> = snapshot
        .removed_files()
        .into_iter()
        .map(map_closure)
        .collect();
    let mut removed_dirs: Vec<_> = snapshot
        .removed_dirs()
        .into_iter()
        .map(map_closure)
        .collect();

    added_files.sort_unstable();
    added_dirs.sort_unstable();
    removed_files.sort_unstable();
    removed_dirs.sort_unstable();

    FsSnapshotVisual {
        added_files,
        added_dirs,
        removed_files,
        removed_dirs,
    }
}
