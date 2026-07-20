use std::process::Command;

use crate::rojo_test::io_util::PRISM_PATH;

#[test]
fn prism_binary_help_is_branded_and_lists_product_commands() {
    let output = Command::new(PRISM_PATH).arg("--help").output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Prism developer tooling for Roblox"));
    for command in ["serve", "exec", "inspect", "plugin"] {
        assert!(stdout.contains(command), "help did not list {command}");
    }
}

#[test]
fn prism_binary_version_is_branded() {
    let output = Command::new(PRISM_PATH).arg("--version").output().unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        format!("Prism {}", env!("CARGO_PKG_VERSION"))
    );
}
