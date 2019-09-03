use std::process::Command;

use crate::util::{get_rojo_path, get_serve_tests_path, get_working_dir_path};

#[test]
fn serve_test() {
    let _ = env_logger::try_init();

    let serve_test_path = get_serve_tests_path();
    let working_dir = get_working_dir_path();

    let input_path = serve_test_path.join("placeholder");

    let exe_path = get_rojo_path();

    let mut handle = Command::new(exe_path)
        .args(&["serve", input_path.to_str().unwrap(), "--port", "35103"])
        .current_dir(working_dir)
        .spawn()
        .expect("Couldn't start Rojo");

    loop {
        let mut response = match reqwest::get("http://localhost:35103/api/rojo") {
            Ok(res) => res,
            Err(err) => {
                log::info!("Server error, retrying: {}", err);
                std::thread::sleep(std::time::Duration::from_millis(30));
                continue;
            }
        };

        let text = response.text().expect("Couldn't get response text");

        log::info!("Got response body: {}", text);
        break;
    }

    handle
        .kill()
        .expect("Rojo server was not running at end of test");
}
