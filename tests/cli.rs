use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "rokf-test-{}-{}",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).expect("create temp test directory");
    dir
}

fn temp_file(name: &str, contents: &str) -> std::path::PathBuf {
    let dir = temp_dir();
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
fn check_accepts_a_healthy_concept_document() {
    let document = temp_file(
        "customers.md",
        "---\ntype: BigQuery Table\ntitle: Customers\ndescription: Customer dimension.\n---\n\n# Customers\n",
    );

    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("check")
        .arg(&document)
        .output()
        .expect("run rokf check");

    assert!(
        output.status.success(),
        "healthy Concept Document should pass; stdout: {}; stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
    assert!(
        stdout.contains("conformant: yes"),
        "stdout should report conformance separately: {stdout}"
    );
    assert!(
        stdout.contains("healthy: yes"),
        "stdout should report health separately: {stdout}"
    );
}

#[test]
fn check_reports_missing_description_as_a_warning_without_breaking_conformance() {
    let document = temp_file(
        "customers.md",
        "---\ntype: BigQuery Table\ntitle: Customers\n---\n\n# Customers\n",
    );

    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("check")
        .arg(&document)
        .output()
        .expect("run rokf check");

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
    assert!(
        stdout.contains("OKF101"),
        "stdout should include missing Description Rule Code: {stdout}"
    );
    assert!(
        stdout.contains("warning"),
        "stdout should report missing recommended fields as warnings: {stdout}"
    );
    assert!(
        stdout.contains("conformant: yes"),
        "Quality Rule Findings should not break conformance: {stdout}"
    );
    assert!(
        stdout.contains("healthy: no"),
        "Quality Rule Findings should affect health: {stdout}"
    );
}

#[test]
fn check_accepts_a_single_concept_document_from_stdin() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("check")
        .arg("-")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("spawn rokf check -");

    std::io::Write::write_all(
        child.stdin.as_mut().expect("stdin is piped"),
        b"---\ntype: BigQuery Table\ndescription: Customer dimension.\n---\n\n# Customers\n",
    )
    .expect("write stdin document");

    let output = child.wait_with_output().expect("wait for rokf check -");

    assert!(
        output.status.success(),
        "stdin Concept Document should pass; stdout: {}; stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn check_traverses_nested_concept_documents_in_a_bundle_root() {
    let bundle = temp_dir();
    let squad = bundle.join("squads");
    fs::create_dir_all(&squad).expect("create nested bundle hierarchy");
    fs::write(
        bundle.join("captain-rex.md"),
        "---\ntype: Person\ndescription: Clone captain.\n---\n\n# Captain Rex\n",
    )
    .expect("write conformant concept document");
    fs::write(squad.join("torrent-company.md"), "# Torrent Company\n")
        .expect("write non-conformant nested concept document");

    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("check")
        .arg(&bundle)
        .output()
        .expect("run rokf check bundle");

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
    assert!(
        stdout.contains("OKF001"),
        "stdout should include nested Concept Document Finding: {stdout}"
    );
    assert!(
        stdout.contains(&squad.join("torrent-company.md").display().to_string()),
        "stdout should include nested OKF Document identity: {stdout}"
    );
    assert!(
        !stdout.contains("captain-rex.md"),
        "stdout should not report conformant Concept Documents: {stdout}"
    );
}

#[test]
fn check_classifies_reserved_files_separately_from_concept_documents_in_a_bundle_root() {
    let bundle = temp_dir();
    fs::write(bundle.join("index.md"), "# Clone Intelligence\n").expect("write Root Index File");
    fs::write(bundle.join("log.md"), "# Log\n").expect("write Log File");
    fs::write(
        bundle.join("phase-ii-armor.md"),
        "---\ntype: Equipment\ndescription: Clone trooper armor.\n---\n\n# Phase II Armor\n",
    )
    .expect("write conformant concept document");

    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("check")
        .arg(&bundle)
        .output()
        .expect("run rokf check bundle");

    assert!(
        output.status.success(),
        "Reserved Files should not be checked as Concept Documents; stdout: {}; stderr: {}",
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
