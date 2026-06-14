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
fn check_can_keep_warnings_visible_without_failing_at_error_threshold() {
    let document = temp_file(
        "customers.md",
        "---\ntype: BigQuery Table\ntitle: Customers\n---\n\n# Customers\n",
    );

    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("check")
        .arg("--failure-threshold")
        .arg("error")
        .arg(&document)
        .output()
        .expect("run rokf check with error Failure Threshold");

    assert!(
        output.status.success(),
        "warnings should not fail at the Error Failure Threshold; stdout: {}; stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
    assert!(
        stdout.contains("OKF101"),
        "findings below the Failure Threshold should remain visible: {stdout}"
    );
    assert!(
        stdout.contains("conformant: yes"),
        "Failure Threshold should not redefine conformance: {stdout}"
    );
    assert!(
        stdout.contains("healthy: yes"),
        "warnings below the Failure Threshold should not make the document unhealthy: {stdout}"
    );
}

#[test]
fn check_warning_findings_fail_at_suggestion_threshold() {
    let document = temp_file(
        "customers.md",
        "---\ntype: BigQuery Table\ntitle: Customers\n---\n\n# Customers\n",
    );

    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("check")
        .arg("--failure-threshold")
        .arg("suggestion")
        .arg(&document)
        .output()
        .expect("run rokf check with suggestion Failure Threshold");

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
    assert!(
        stdout.contains("OKF101"),
        "warnings should remain visible at the Suggestion Failure Threshold: {stdout}"
    );
}

#[test]
fn check_applies_failure_threshold_to_stdin_documents() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("check")
        .arg("--failure-threshold")
        .arg("error")
        .arg("-")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("spawn rokf check - with error Failure Threshold");

    std::io::Write::write_all(
        child.stdin.as_mut().expect("stdin is piped"),
        b"---\ntype: BigQuery Table\n---\n\n# Customers\n",
    )
    .expect("write stdin document");

    let output = child.wait_with_output().expect("wait for rokf check -");

    assert!(
        output.status.success(),
        "stdin warnings should not fail at the Error Failure Threshold; stdout: {}; stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
    assert!(
        stdout.contains("OKF101"),
        "stdin findings below the Failure Threshold should remain visible: {stdout}"
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
fn check_explicit_bundle_root_bypasses_bundle_discovery() {
    let outside_dir = temp_dir();
    let bundle = temp_dir();
    fs::write(
        outside_dir.join("orphan-concept.md"),
        "---\ntype: Equipment\ndescription: Outside bundle.\n---\n",
    )
    .expect("write concept document outside bundle");
    fs::write(bundle.join("index.md"), "# Bundle Index\n").expect("write Root Index File");
    fs::write(
        bundle.join("log.md"),
        "# Bundle Update Log\n\n## 2026-06-13\n* **Creation**: Initialized bundle.\n",
    )
    .expect("write Log File");
    fs::write(bundle.join("kix.md"), "# Kix\n").expect("write non-conformant concept document");

    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("check")
        .arg(&bundle)
        .current_dir(&outside_dir)
        .output()
        .expect("run rokf check with explicit bundle root from outside directory");

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
    assert!(
        stdout.contains("OKF001"),
        "explicit Bundle Root should still verify the bundle: {stdout}"
    );
    assert!(
        stdout.contains(&bundle.join("kix.md").display().to_string()),
        "stdout should include explicit Bundle Root content: {stdout}"
    );
}

#[test]
fn check_stdin_bypasses_bundle_discovery_when_no_bundle_root_is_supplied() {
    let dir = temp_dir();
    fs::write(
        dir.join("orphan-concept.md"),
        "---\ntype: Equipment\ndescription: No bundle marker.\n---\n",
    )
    .expect("write concept document without a bundle marker");

    let mut child = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("check")
        .arg("-")
        .current_dir(&dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("spawn rokf check - without a discoverable bundle root");

    std::io::Write::write_all(
        child.stdin.as_mut().expect("stdin is piped"),
        b"---\ntype: Person\ndescription: Clone captain.\n---\n\n# Captain Rex\n",
    )
    .expect("write stdin document");

    let output = child.wait_with_output().expect("wait for rokf check -");

    assert!(
        output.status.success(),
        "stdin input should bypass Bundle Discovery and pass; stdout: {}; stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn check_reports_failed_bundle_discovery_with_a_clear_error_and_deterministic_exit_code() {
    let dir = temp_dir();
    fs::write(
        dir.join("orphan-concept.md"),
        "---\ntype: Equipment\ndescription: No bundle marker.\n---\n",
    )
    .expect("write concept document without a bundle marker");

    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("check")
        .current_dir(&dir)
        .output()
        .expect("run rokf check without a discoverable bundle root");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("stderr is utf-8");
    assert!(
        stderr.contains("could not discover"),
        "stderr should report failed discovery clearly: {stderr}"
    );
}

#[test]
fn check_discovers_the_nearest_bundle_root_from_a_nested_directory() {
    let bundle = temp_dir();
    fs::write(bundle.join("index.md"), "# Bundle Index\n").expect("write Root Index File");
    let nested = bundle.join("squads");
    fs::create_dir_all(&nested).expect("create nested directory");
    fs::write(nested.join("torrent-company.md"), "# Torrent Company\n")
        .expect("write non-conformant nested concept document");

    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("check")
        .current_dir(&nested)
        .output()
        .expect("run rokf check from nested directory");

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
    assert!(
        stdout.contains("OKF001"),
        "Bundle Discovery should traverse up from the nested directory: {stdout}"
    );
    assert!(
        stdout.contains(&nested.join("torrent-company.md").display().to_string()),
        "stdout should include nested OKF Document identity: {stdout}"
    );
}

#[test]
fn check_discovers_a_bundle_root_from_the_current_directory() {
    let bundle = temp_dir();
    fs::write(bundle.join("index.md"), "# Bundle Index\n").expect("write Root Index File");
    fs::write(bundle.join("captain-rex.md"), "# Captain Rex\n")
        .expect("write non-conformant concept document");

    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("check")
        .current_dir(&bundle)
        .output()
        .expect("run rokf check from bundle root");

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
    assert!(
        stdout.contains("OKF001"),
        "Bundle Discovery should find the Bundle Root and verify it: {stdout}"
    );
    assert!(
        stdout.contains(&bundle.join("captain-rex.md").display().to_string()),
        "stdout should include discovered Bundle Root content: {stdout}"
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

#[test]
fn check_validates_reserved_files_in_a_fixture_bundle() {
    let bundle = temp_dir();
    let concepts = bundle.join("concepts");
    fs::create_dir_all(&concepts).expect("create nested bundle hierarchy");

    fs::write(
        bundle.join("index.md"),
        "---\nokf_version: \"0.1\"\n---\n\n# Bundle Index\n\n* [Concepts](concepts/) — Directory of concepts\n",
    )
    .expect("write Root Index File");
    fs::write(
        bundle.join("log.md"),
        "# Update Log\n\n## 2026-06-13\n* **Creation**: Initialized bundle.\n",
    )
    .expect("write Log File");
    fs::write(
        concepts.join("index.md"),
        "# Concepts\n\n* [Widget](widget.md) — A representative concept\n",
    )
    .expect("write nested Index File");
    fs::write(
        concepts.join("widget.md"),
        "---\ntype: Concept\ndescription: A representative concept.\n---\n\n# Widget\n",
    )
    .expect("write conformant Concept Document");

    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("check")
        .arg(&bundle)
        .output()
        .expect("run rokf check bundle");

    assert!(
        output.status.success(),
        "fixture bundle should be conformant and healthy; stdout: {}; stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
    assert!(
        stdout.contains("conformant: yes"),
        "fixture bundle should be conformant: {stdout}"
    );
    assert!(
        stdout.contains("healthy: yes"),
        "fixture bundle should be healthy: {stdout}"
    );
    assert!(
        !stdout.contains("OKF200"),
        "nested Index Files should not contain frontmatter: {stdout}"
    );
    assert!(
        !stdout.contains("OKF201"),
        "Log Files should not contain frontmatter: {stdout}"
    );
}

#[test]
fn check_reports_index_file_frontmatter_as_an_error() {
    let bundle = temp_dir();
    fs::write(bundle.join("index.md"), "# Root Index\n").expect("write Root Index File");
    let nested = bundle.join("nested");
    fs::create_dir_all(&nested).expect("create nested directory");
    fs::write(
        nested.join("index.md"),
        "---\nokf_version: \"0.1\"\n---\n\n# Nested Index\n",
    )
    .expect("write nested Index File with frontmatter");

    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("check")
        .arg(&bundle)
        .output()
        .expect("run rokf check bundle");

    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
    assert!(
        stdout.contains("OKF200"),
        "stdout should include Index File frontmatter Rule Code: {stdout}"
    );
    assert!(
        stdout.contains("error"),
        "stdout should report Index File frontmatter as an error: {stdout}"
    );
    assert!(
        stdout.contains("conformant: no"),
        "Index File frontmatter should break conformance: {stdout}"
    );
}

#[test]
fn check_reports_log_file_frontmatter_as_an_error() {
    let bundle = temp_dir();
    fs::write(bundle.join("index.md"), "# Root Index\n").expect("write Root Index File");
    fs::write(
        bundle.join("log.md"),
        "---\ntype: Log\n---\n\n# Update Log\n",
    )
    .expect("write Log File with frontmatter");

    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("check")
        .arg(&bundle)
        .output()
        .expect("run rokf check bundle");

    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
    assert!(
        stdout.contains("OKF201"),
        "stdout should include Log File frontmatter Rule Code: {stdout}"
    );
    assert!(
        stdout.contains("error"),
        "stdout should report Log File frontmatter as an error: {stdout}"
    );
    assert!(
        stdout.contains("conformant: no"),
        "Log File frontmatter should break conformance: {stdout}"
    );
}

#[test]
fn check_reports_malformed_root_index_frontmatter_as_an_error() {
    let bundle = temp_dir();
    fs::write(
        bundle.join("index.md"),
        "---\nokf_version: \"0.1\"\nnot yaml: [
---\n\n# Root Index\n",
    )
    .expect("write Root Index File with malformed frontmatter");

    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("check")
        .arg(&bundle)
        .output()
        .expect("run rokf check bundle");

    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
    assert!(
        stdout.contains("OKF202"),
        "stdout should include Root Index File frontmatter Rule Code: {stdout}"
    );
    assert!(
        stdout.contains("error"),
        "stdout should report malformed Root Index File frontmatter as an error: {stdout}"
    );
}

#[test]
fn check_reports_unknown_okf_version_as_a_warning() {
    let bundle = temp_dir();
    fs::write(
        bundle.join("index.md"),
        "---\nokf_version: \"9.9\"\n---\n\n# Root Index\n",
    )
    .expect("write Root Index File with unknown version");

    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("check")
        .arg("--failure-threshold")
        .arg("error")
        .arg(&bundle)
        .output()
        .expect("run rokf check bundle with error Failure Threshold");

    assert!(
        output.status.success(),
        "unknown OKF versions should not fail Verification: stdout: {}; stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
    assert!(
        stdout.contains("OKF203"),
        "stdout should include unknown version Rule Code: {stdout}"
    );
    assert!(
        stdout.contains("warning"),
        "stdout should report unknown version as a warning: {stdout}"
    );
    assert!(
        stdout.contains("conformant: yes"),
        "unknown version should not break conformance: {stdout}"
    );
}
