mod test_util;

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    thread,
    time::Duration,
};

use tempfile::{tempdir, TempDir};

use librojo::{
    live_session::LiveSession,
    project::Project,
};

use crate::test_util::{
    copy_recursive,
    tree::tree_step,
};

#[test]
fn multi_partition_game() {
    let _ = env_logger::try_init();
    let source_path = project_path("multi_partition_game");

    let (dir, live_session) = start_session(&source_path);
    tree_step("initial", &live_session, &source_path);

    let added_path = dir.path().join("a/added");
    fs::create_dir_all(&added_path)
        .expect("Couldn't create directory");
    thread::sleep(Duration::from_millis(250));

    tree_step("with_dir", &live_session, &source_path);

    let moved_path = dir.path().join("b/added");
    fs::rename(&added_path, &moved_path)
        .expect("Couldn't rename directory");
    thread::sleep(Duration::from_millis(250));

    tree_step("with_moved_dir", &live_session, &source_path);
}

/// Find the path to the given test project relative to the manifest.
fn project_path(name: &str) -> PathBuf {
    let mut path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-projects");
    path.push(name);
    path
}

/// Starts a new LiveSession for the project located at the given file path.
fn start_session(source_path: &Path) -> (TempDir, LiveSession) {
    let dir = tempdir()
        .expect("Couldn't create temporary directory");

    copy_recursive(&source_path, dir.path())
        .expect("Couldn't copy project to temporary directory");

    let project = Arc::new(Project::load_fuzzy(dir.path())
        .expect("Couldn't load project from temp directory"));

    let live_session = LiveSession::new(Arc::clone(&project))
        .expect("Couldn't start live session");

    (dir, live_session)
}