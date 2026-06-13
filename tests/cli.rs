use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_file(name: &str, contents: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "rokf-test-{}-{}",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).expect("create temp test directory");
    let path = dir.join(name);
    fs::write(&path, contents).expect("write temp test document");
    path
}

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
fn check_reports_missing_frontmatter_as_an_error_finding() {
    let document = temp_file("customers.md", "# Customers\n");

    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("check")
        .arg(&document)
        .output()
        .expect("run rokf check");

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
    assert!(
        stdout.contains("OKF001"),
        "stdout should include Rule Code: {stdout}"
    );
    assert!(
        stdout.contains("error"),
        "stdout should include Severity: {stdout}"
    );
    assert!(
        stdout.contains(&document.display().to_string()),
        "stdout should include OKF Document identity: {stdout}"
    );
    assert!(
        stdout.contains(":1:1"),
        "stdout should include location: {stdout}"
    );
}

#[test]
fn check_reports_missing_concept_type_as_an_error_finding() {
    let document = temp_file(
        "customers.md",
        "---\ntitle: Customers\n---\n\n# Customers\n",
    );

    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("check")
        .arg(&document)
        .output()
        .expect("run rokf check");

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
    assert!(
        stdout.contains("OKF002"),
        "stdout should include Rule Code: {stdout}"
    );
    assert!(
        stdout.contains("error"),
        "stdout should include Severity: {stdout}"
    );
    assert!(
        stdout.contains(&document.display().to_string()),
        "stdout should include OKF Document identity: {stdout}"
    );
}

#[test]
fn check_reports_malformed_frontmatter_as_an_error_finding() {
    let document = temp_file(
        "customers.md",
        "---\ntype: BigQuery Table\ntags: [customers\n---\n\n# Customers\n",
    );

    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("check")
        .arg(&document)
        .output()
        .expect("run rokf check");

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
    assert!(
        stdout.contains("OKF001"),
        "stdout should include Rule Code: {stdout}"
    );
    assert!(
        stdout.contains("error"),
        "stdout should include Severity: {stdout}"
    );
}

#[test]
fn check_accepts_a_conformant_concept_document() {
    let document = temp_file(
        "customers.md",
        "---\ntype: BigQuery Table\ntitle: Customers\n---\n\n# Customers\n",
    );

    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("check")
        .arg(&document)
        .output()
        .expect("run rokf check");

    assert!(
        output.status.success(),
        "conformant Concept Document should pass; stdout: {}; stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
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
