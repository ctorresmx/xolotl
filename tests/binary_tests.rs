use std::process::Command;

#[test]
fn test_binary_help() {
    let output = Command::new("cargo")
        .args(["run", "--bin", "xolotl", "--", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Lightweight, environment-aware service discovery"));
    assert!(stdout.contains("--address"));
    assert!(stdout.contains("--port"));
}

#[test]
fn test_binary_version() {
    let output = Command::new("cargo")
        .args(["run", "--bin", "xolotl", "--", "--version"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("xolotl 0.1.0"));
}
