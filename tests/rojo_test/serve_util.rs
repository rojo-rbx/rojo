use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
    thread,
    time::Duration,
};

use rbx_dom_weak::types::Ref;

use tempfile::{tempdir, TempDir};

use librojo::web_api::{ReadResponse, ServerInfoResponse, SubscribeResponse};
use rojo_insta_ext::RedactionMap;

use crate::rojo_test::io_util::{
    copy_recursive, get_working_dir_path, KillOnDrop, ROJO_PATH, SERVE_TESTS_PATH,
};

/// Convenience method to run a `rojo serve` test.
///
/// Test projects should be defined in the `serve-tests` folder; their filename
/// should be given as the first parameter.
///
/// The passed in callback is where the actual test body should go. Setup and
/// cleanup happens automatically.
pub fn run_serve_test(test_name: &str, callback: impl FnOnce(TestServeSession, RedactionMap)) {
    let _ = env_logger::try_init();

    let mut redactions = RedactionMap::new();

    let mut session = TestServeSession::new(test_name);
    let info = session.wait_to_come_online();

    redactions.intern(info.session_id);
    redactions.intern(info.root_instance_id);

    let mut settings = insta::Settings::new();

    let snapshot_path = Path::new(SERVE_TESTS_PATH)
        .parent()
        .unwrap()
        .join("serve-test-snapshots");

    settings.set_snapshot_path(snapshot_path);
    settings.set_sort_maps(true);
    settings.add_redaction(".serverVersion", "[server-version]");
    settings.bind(move || callback(session, redactions));
}

/// Represents a running Rojo serve session running in a temporary directory.
pub struct TestServeSession {
    // Drop order is important here: we want the process to be killed before the
    // directory it's operating on is destroyed.
    rojo_process: KillOnDrop,
    _dir: TempDir,

    port: usize,
    project_path: PathBuf,
}

impl TestServeSession {
    pub fn new(name: &str) -> Self {
        let working_dir = get_working_dir_path();

        let source_path = Path::new(SERVE_TESTS_PATH).join(name);
        let dir = tempdir().expect("Couldn't create temporary directory");
        let project_path = dir.path().join(name);

        let source_is_file = fs::metadata(&source_path).unwrap().is_file();

        if source_is_file {
            fs::copy(&source_path, &project_path).expect("couldn't copy project file");
        } else {
            fs::create_dir(&project_path).expect("Couldn't create temporary project subdirectory");

            copy_recursive(&source_path, &project_path)
                .expect("Couldn't copy project to temporary directory");
        };

        let port = get_port_number();
        let port_string = port.to_string();

        let rojo_process = Command::new(ROJO_PATH)
            .args(&[
                "serve",
                project_path.to_str().unwrap(),
                "--port",
                port_string.as_str(),
            ])
            .current_dir(working_dir)
            .spawn()
            .expect("Couldn't start Rojo");

        TestServeSession {
            rojo_process: KillOnDrop(rojo_process),
            _dir: dir,
            port,
            project_path,
        }
    }

    pub fn path(&self) -> &Path {
        &self.project_path
    }

    /// Waits for the `rojo serve` server to come online with expontential
    /// backoff.
    pub fn wait_to_come_online(&mut self) -> ServerInfoResponse {
        const BASE_DURATION_MS: f32 = 30.0;
        const EXP_BACKOFF_FACTOR: f32 = 1.3;
        const MAX_TRIES: u32 = 5;

        for i in 1..=MAX_TRIES {
            match self.rojo_process.0.try_wait() {
                Ok(Some(status)) => panic!("Rojo process exited with status {}", status),
                Ok(None) => { /* The process is still running, as expected */ }
                Err(err) => panic!("Failed to wait on Rojo process: {}", err),
            }

            let info = match self.get_api_rojo() {
                Ok(info) => info,
                Err(err) => {
                    let retry_time_ms = BASE_DURATION_MS * (i as f32).powf(EXP_BACKOFF_FACTOR);
                    let retry_time = Duration::from_millis(retry_time_ms as u64);

                    log::info!("Server error, retrying in {:?}: {}", retry_time, err);
                    thread::sleep(retry_time);
                    continue;
                }
            };

            log::info!("Got session info: {:?}", info);

            return info;
        }

        panic!("Rojo server did not respond after {} tries.", MAX_TRIES);
    }

    pub fn get_api_rojo(&self) -> Result<ServerInfoResponse, reqwest::Error> {
        let url = format!("http://localhost:{}/api/rojo", self.port);
        let body = reqwest::blocking::get(&url)?.text()?;

        Ok(serde_json::from_str(&body).expect("Server returned malformed response"))
    }

    pub fn get_api_read(&self, id: Ref) -> Result<ReadResponse, reqwest::Error> {
        let url = format!("http://localhost:{}/api/read/{}", self.port, id);
        let body = reqwest::blocking::get(&url)?.text()?;

        Ok(serde_json::from_str(&body).expect("Server returned malformed response"))
    }

    pub fn get_api_subscribe(
        &self,
        cursor: u32,
    ) -> Result<SubscribeResponse<'static>, reqwest::Error> {
        let url = format!("http://localhost:{}/api/subscribe/{}", self.port, cursor);

        reqwest::blocking::get(&url)?.json()
    }
}

/// Probably-okay way to generate random enough port numbers for running the
/// Rojo live server.
///
/// If this method ends up having problems, we should add an option for Rojo to
/// use a random port chosen by the operating system and figure out a good way
/// to get that port back to the test CLI.
fn get_port_number() -> usize {
    static NEXT_PORT_NUMBER: AtomicUsize = AtomicUsize::new(35103);

    NEXT_PORT_NUMBER.fetch_add(1, Ordering::SeqCst)
}
