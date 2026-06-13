use std::process::Command;

#[test]
fn help_describes_rokf_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("--help")
        .output()
        .expect("run rokf --help");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("help is utf-8");
    assert!(
        stdout.contains("rokf"),
        "help should include binary name: {stdout}"
    );
    assert!(
        stdout.contains("Open Knowledge Format"),
        "help should describe the Open Knowledge Format workflow: {stdout}"
    );
}

#[test]
fn version_reports_cargo_package_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("--version")
        .output()
        .expect("run rokf --version");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("version is utf-8");
    assert!(
        stdout.contains(env!("CARGO_PKG_VERSION")),
        "version should include Cargo package version: {stdout}"
    );
}
