use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

use insta::assert_yaml_snapshot;
use rbx_dom_weak::RbxId;
use tempfile::{tempdir, TempDir};

use librojo::web_interface::{ReadResponse, ServerInfoResponse};

use crate::util::{
    copy_recursive, get_rojo_path, get_serve_tests_path, get_working_dir_path, KillOnDrop,
};

#[test]
fn empty() {
    run_serve_test(|session, mut dm| {
        let info = session.get_api_rojo().unwrap();

        let root_id = info.root_instance_id;

        let mut info = serde_yaml::to_value(info).unwrap();
        dm.redact(&mut info);

        assert_yaml_snapshot!(info);

        let read_result = session.get_api_read(root_id).unwrap();

        dm.intern_iter(read_result.instances.keys().copied());

        let mut read_result = serde_yaml::to_value(read_result).unwrap();
        dm.redact(&mut read_result);

        assert_yaml_snapshot!(read_result);
    });
}

fn run_serve_test(callback: impl FnOnce(TestServeSession, DeterMap)) {
    let _ = env_logger::try_init();

    let mut settings = insta::Settings::new();

    let snapshot_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("serve-test-snapshots");
    settings.set_snapshot_path(snapshot_path);

    let mut dm = DeterMap::new();

    let mut session = TestServeSession::new("empty");
    let info = session.wait_to_come_online();

    dm.intern(info.session_id);
    dm.intern(info.root_instance_id);

    settings.bind(move || callback(session, dm));
}

struct DeterMap {
    ids: HashMap<String, usize>,
    last_id: usize,
}

impl DeterMap {
    fn new() -> DeterMap {
        DeterMap {
            ids: HashMap::new(),
            last_id: 0,
        }
    }

    fn intern(&mut self, id: impl ToString) {
        let last_id = &mut self.last_id;

        self.ids.entry(id.to_string()).or_insert_with(|| {
            *last_id += 1;
            *last_id
        });
    }

    fn intern_iter<S: ToString>(&mut self, ids: impl Iterator<Item = S>) {
        for id in ids {
            self.intern(id.to_string());
        }
    }

    fn redact(&self, yaml_value: &mut serde_yaml::Value) {
        use serde_yaml::{Mapping, Value};

        match yaml_value {
            Value::String(value) => {
                if let Some(redacted) = self.ids.get(value) {
                    *yaml_value = Value::String(format!("id-{}", *redacted));
                }
            }
            Value::Sequence(sequence) => {
                for value in sequence {
                    self.redact(value);
                }
            }
            Value::Mapping(mapping) => {
                // We can't mutate the keys of a map in-place, so we take
                // ownership of the map and rebuild it.

                let owned_map = std::mem::replace(mapping, Mapping::new());
                let mut new_map = Mapping::with_capacity(owned_map.len());

                for (mut key, mut value) in owned_map {
                    self.redact(&mut key);
                    self.redact(&mut value);
                    new_map.insert(key, value);
                }

                *mapping = new_map;
            }
            _ => {}
        }
    }
}

fn get_port_number() -> usize {
    static NEXT_PORT_NUMBER: AtomicUsize = AtomicUsize::new(35103);

    NEXT_PORT_NUMBER.fetch_add(1, Ordering::SeqCst)
}

struct TestServeSession {
    // Drop order is important here: we want the process to be killed before the
    // directory it's operating on is destroyed.
    rojo_process: KillOnDrop,
    _dir: TempDir,

    port: usize,
    project_path: PathBuf,
}

impl TestServeSession {
    pub fn new(name: &str) -> Self {
        let serve_test_path = get_serve_tests_path();
        let working_dir = get_working_dir_path();
        let exe_path = get_rojo_path();

        let source_path = serve_test_path.join(name);
        let dir = tempdir().expect("Couldn't create temporary directory");
        let project_path = dir.path().join(name);

        fs::create_dir(&project_path).expect("Couldn't create temporary project subdirectory");

        copy_recursive(&source_path, &project_path)
            .expect("Couldn't copy project to temporary directory");

        let port = get_port_number();
        let port_string = port.to_string();

        let rojo_process = Command::new(exe_path)
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

    pub fn wait_to_come_online(&mut self) -> ServerInfoResponse {
        loop {
            match self.rojo_process.0.try_wait() {
                Ok(Some(status)) => panic!("Rojo process exited with status {}", status),
                Ok(None) => { /* process is still running */ }
                Err(err) => panic!("Failed to wait on Rojo process: {}", err),
            }

            let info = match self.get_api_rojo() {
                Ok(info) => info,
                Err(err) => {
                    log::debug!("Server error, retrying: {}", err);
                    std::thread::sleep(std::time::Duration::from_millis(30));
                    continue;
                }
            };

            log::info!("Got session info: {:?}", info);

            return info;
        }
    }

    pub fn get_api_rojo(&self) -> Result<ServerInfoResponse, reqwest::Error> {
        let url = format!("http://localhost:{}/api/rojo", self.port);
        let body = reqwest::get(&url)?.text()?;

        Ok(serde_json::from_str(&body).expect("Server returned malformed response"))
    }

    pub fn get_api_read(&self, id: RbxId) -> Result<ReadResponse, reqwest::Error> {
        let url = format!("http://localhost:{}/api/read/{}", self.port, id);
        let body = reqwest::get(&url)?.text()?;

        Ok(serde_json::from_str(&body).expect("Server returned malformed response"))
    }
}
