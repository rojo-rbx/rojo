use std::{
    path::Path,
    process::Command,
    thread,
    time::Duration,
};

fn main() {
    let plugin_path = Path::new("../plugin");
    let server_path = Path::new("../server");
    let tests_path = Path::new("../tests");

    let server = Command::new("cargo")
        .args(&["run", "--", "serve", "../test-projects/empty"])
        .current_dir(server_path)
        .spawn();

    thread::sleep(Duration::from_millis(1000));

    // TODO: Wait for server to start responding on the right port

    let test_client = Command::new("lua")
        .args(&["runTest.lua", "tests/empty.lua"])
        .current_dir(plugin_path)
        .spawn();

    thread::sleep(Duration::from_millis(300));

    // TODO: Collect output from the client for success/failure?

    println!("Dying!");
}
