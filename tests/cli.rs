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

fn write_stdin(args: &[&str], input: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("spawn rokf with stdin");

    std::io::Write::write_all(
        child.stdin.as_mut().expect("stdin is piped"),
        input.as_bytes(),
    )
    .expect("write stdin");

    child.wait_with_output().expect("wait for rokf")
}

fn parse_json_stdout(output: std::process::Output) -> serde_yaml::Mapping {
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
    serde_yaml::from_str(&stdout).expect("stdout is parseable JSON")
}

fn yaml_key(key: &str) -> serde_yaml::Value {
    serde_yaml::Value::String(key.to_string())
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
fn check_reports_unclosed_frontmatter_without_panicking() {
    let document = temp_file("customers.md", "---");

    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("check")
        .arg(&document)
        .output()
        .expect("run rokf check");

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
    assert!(
        stdout.contains("OKF001"),
        "stdout should include Frontmatter Rule Code: {stdout}"
    );
    assert!(
        stdout.contains("closing --- delimiter"),
        "stdout should describe the unclosed Frontmatter: {stdout}"
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

#[test]
fn check_outputs_json_for_a_single_document() {
    let document = temp_file("customers.md", "---\ntype: Table\n---\n\n# Customers\n");

    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("check")
        .arg("--output")
        .arg("json")
        .arg(&document)
        .output()
        .expect("run rokf check with JSON output");

    assert_eq!(output.status.code(), Some(1));
    let report = parse_json_stdout(output);
    assert_eq!(
        report
            .get(yaml_key("conformant"))
            .and_then(|value| value.as_bool()),
        Some(true)
    );
    assert_eq!(
        report
            .get(yaml_key("healthy"))
            .and_then(|value| value.as_bool()),
        Some(false)
    );
    assert_eq!(
        report
            .get(yaml_key("status"))
            .and_then(|value| value.as_str()),
        Some("fail")
    );

    let findings = report
        .get(yaml_key("findings"))
        .and_then(|v| v.as_sequence())
        .expect("JSON output includes Findings");
    let finding = findings.first().expect("JSON output includes one Finding");
    let finding = finding.as_mapping().expect("Finding is an object");
    assert_eq!(
        finding
            .get(yaml_key("rule_code"))
            .and_then(|value| value.as_str()),
        Some("OKF101")
    );
    assert_eq!(
        finding
            .get(yaml_key("severity"))
            .and_then(|value| value.as_str()),
        Some("warning")
    );
    let document_identity = document.display().to_string();
    assert_eq!(
        finding
            .get(yaml_key("document"))
            .and_then(|value| value.as_str()),
        Some(document_identity.as_str())
    );
}

#[test]
fn check_outputs_json_for_stdin_and_bundle_documents() {
    let stdin_output = write_stdin(
        &["check", "--output", "json", "-"],
        "---\ntype: Table\ndescription: Customers.\n---\n",
    );

    assert!(stdin_output.status.success());
    let stdin_report = parse_json_stdout(stdin_output);
    assert_eq!(
        stdin_report
            .get(yaml_key("status"))
            .and_then(|value| value.as_str()),
        Some("pass")
    );

    let bundle = temp_dir();
    fs::write(bundle.join("index.md"), "# Bundle Index\n").expect("write Root Index File");
    fs::write(
        bundle.join("orders.md"),
        "---\ntype: Table\ndescription: Orders.\n---\n\nSee [Customers](/customers.md).\n",
    )
    .expect("write concept with broken link");

    let bundle_output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("check")
        .arg("--output")
        .arg("json")
        .arg("--failure-threshold")
        .arg("error")
        .arg(&bundle)
        .output()
        .expect("run bundle check with JSON output");

    assert!(bundle_output.status.success());
    let bundle_report = parse_json_stdout(bundle_output);
    let findings = bundle_report
        .get(yaml_key("findings"))
        .and_then(|value| value.as_sequence())
        .expect("Bundle JSON output includes Findings");
    let finding = findings
        .first()
        .expect("Bundle JSON output includes a Finding");
    let finding = finding.as_mapping().expect("Finding is an object");
    assert_eq!(
        finding
            .get(yaml_key("rule_code"))
            .and_then(|value| value.as_str()),
        Some("OKF400")
    );
}

#[test]
fn check_fix_sorts_tags_and_preserves_unknown_fields_and_body() {
    let document = temp_file(
        "customers.md",
        concat!(
            "---\n",
            "type: Table\n",
            "owner: analytics\n",
            "tags: [zeta, alpha]\n",
            "---\n\n",
            "# Customers\n\n",
            "Body stays.\n",
        ),
    );

    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("check")
        .arg("--fix")
        .arg(&document)
        .output()
        .expect("run rokf check --fix");

    assert_eq!(output.status.code(), Some(1));
    let contents = fs::read_to_string(&document).expect("read fixed document");
    assert!(
        contents.contains("owner: analytics"),
        "Producer-defined Fields should survive fixes: {contents}"
    );
    assert!(
        contents.contains("tags: [alpha, zeta]"),
        "fix should sort tags: {contents}"
    );
    assert!(
        contents.contains("# Customers\n\nBody stays."),
        "Body should survive fixes: {contents}"
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
    assert!(
        stdout.contains("OKF101"),
        "non-fixable Findings should remain visible: {stdout}"
    );
}

#[test]
fn check_fix_from_stdin_writes_fixed_document_to_stdout() {
    let output = write_stdin(
        &["check", "--fix", "-"],
        "---\ntype: Table\ntags: [zeta, alpha]\ndescription: Customers.\n---\n\n# Customers\n",
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
    assert!(
        stdout.contains("tags: [alpha, zeta]"),
        "stdin fixing should write the fixed document to stdout: {stdout}"
    );
}

#[test]
fn format_check_reports_drift_without_mutating_documents() {
    let document = temp_file(
        "customers.md",
        "---\ntype: Table\ndescription: Customers.   \n---\n\n# Customers\n",
    );

    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("format")
        .arg("--check")
        .arg(&document)
        .output()
        .expect("run rokf format --check");

    assert_eq!(output.status.code(), Some(1));
    let contents = fs::read_to_string(&document).expect("read document after format check");
    assert!(
        contents.contains("Customers.   "),
        "format --check should not mutate documents: {contents}"
    );
}

#[test]
fn format_normalizes_stdin_without_changing_body_text() {
    let output = write_stdin(
        &["format", "-"],
        "---\ntype: Table\ndescription: Customers.   \n---\n\n# Customers\nBody   ",
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
    assert_eq!(
        stdout,
        "---\ntype: Table\ndescription: Customers.\n---\n\n# Customers\nBody\n"
    );
}

#[test]
fn format_check_reports_stdin_drift_without_printing_formatted_content() {
    let output = write_stdin(
        &["format", "--check", "-"],
        "---\ntype: Table\ndescription: Customers.   \n---\n",
    );

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
    assert!(
        stdout.contains("<stdin>: would reformat"),
        "stdin format check should report drift: {stdout}"
    );
    assert!(
        !stdout.contains("description: Customers.\n"),
        "format --check should not print formatted stdin content: {stdout}"
    );
}

#[test]
fn bundle_check_reports_broken_concept_links_as_warnings() {
    let bundle = temp_dir();
    fs::write(bundle.join("index.md"), "# Bundle Index\n").expect("write Root Index File");
    fs::write(
        bundle.join("orders.md"),
        concat!(
            "---\n",
            "type: Table\n",
            "description: Orders.\n",
            "---\n\n",
            "See [Customers](/customers.md) and [Docs](https://example.com).\n",
        ),
    )
    .expect("write concept with broken link");

    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("check")
        .arg("--failure-threshold")
        .arg("error")
        .arg(&bundle)
        .output()
        .expect("run rokf check bundle");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
    assert!(
        stdout.contains("OKF400") && stdout.contains("warning"),
        "Broken Links should be bundle-level warnings: {stdout}"
    );
    assert!(
        !stdout.contains("example.com"),
        "external links should not be treated as Broken Links: {stdout}"
    );
}

#[test]
fn bundle_check_resolves_relative_and_bundle_relative_concept_links() {
    let bundle = temp_dir();
    let catalog = bundle.join("catalog");
    fs::create_dir_all(&catalog).expect("create nested catalog");
    fs::write(bundle.join("index.md"), "# Bundle Index\n").expect("write Root Index File");
    fs::write(
        bundle.join("customers.md"),
        "---\ntype: Table\ndescription: Customers.\n---\n",
    )
    .expect("write bundle-relative target");
    fs::write(
        catalog.join("inventory.md"),
        "---\ntype: Table\ndescription: Inventory.\n---\n",
    )
    .expect("write relative target");
    fs::write(
        catalog.join("orders.md"),
        concat!(
            "---\n",
            "type: Table\n",
            "description: Orders.\n",
            "---\n\n",
            "See [Inventory](inventory.md) and [Customers](/customers.md).\n",
        ),
    )
    .expect("write concept links");

    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("check")
        .arg(&bundle)
        .output()
        .expect("run rokf check bundle");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
    assert!(
        !stdout.contains("OKF400"),
        "valid Concept Links should not be reported as Broken Links: {stdout}"
    );
}

#[test]
fn document_check_does_not_report_broken_links_in_isolation() {
    let document = temp_file(
        "orders.md",
        "---\ntype: Table\ndescription: Orders.\n---\n\nSee [Customers](/customers.md).\n",
    );

    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("check")
        .arg(&document)
        .output()
        .expect("run rokf check document");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
    assert!(
        !stdout.contains("OKF400"),
        "isolated Document Verification should skip bundle links: {stdout}"
    );
}

#[test]
fn configuration_applies_rule_set_suppressions_exclusions_and_threshold() {
    let bundle = temp_dir();
    fs::write(bundle.join("index.md"), "# Bundle Index\n").expect("write Root Index File");
    fs::write(
        bundle.join("rokf.yml"),
        concat!(
            "failure_threshold: error\n",
            "rule_set: default\n",
            "suppressions:\n",
            "  - OKF101\n",
            "exclusions:\n",
            "  - excluded.md\n",
        ),
    )
    .expect("write configuration");
    fs::write(bundle.join("included.md"), "---\ntype: Concept\n---\n").expect("write included");
    fs::write(bundle.join("excluded.md"), "# Excluded\n").expect("write excluded");

    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("check")
        .arg(&bundle)
        .output()
        .expect("run configured rokf check");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
    assert!(
        !stdout.contains("OKF101") && !stdout.contains("excluded.md"),
        "configuration should suppress and exclude configured content: {stdout}"
    );
}

#[test]
fn configuration_applies_to_single_document_checks() {
    let dir = temp_dir();
    let document = dir.join("concept.md");
    fs::write(
        dir.join("rokf.yml"),
        "failure_threshold: warning\nsuppressions:\n  - OKF101\n",
    )
    .expect("write configuration");
    fs::write(&document, "---\ntype: Concept\n---\n").expect("write Concept Document");

    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("check")
        .arg(&document)
        .output()
        .expect("run configured document check");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
    assert!(
        !stdout.contains("OKF101"),
        "single-document Verification should apply configured Suppressions: {stdout}"
    );
}

#[test]
fn explicit_configuration_applies_to_stdin_checks() {
    let config = temp_file(
        "rokf.yml",
        "failure_threshold: error\nrule_set: conformance\n",
    );

    let output = write_stdin(
        &[
            "check",
            "--config",
            config.to_str().expect("config path is UTF-8"),
            "-",
        ],
        "---\ntitle: Untyped\n---\n",
    );

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
    assert!(
        stdout.contains("OKF002"),
        "stdin Verification should keep configured Conformance Rule Findings: {stdout}"
    );
    assert!(
        !stdout.contains("OKF101"),
        "stdin Verification should apply configured Rule Sets: {stdout}"
    );
}

#[test]
fn conformance_rule_set_reports_conformance_rules_only() {
    let bundle = temp_dir();
    fs::write(bundle.join("index.md"), "# Bundle Index\n").expect("write Root Index File");
    fs::write(
        bundle.join("rokf.yml"),
        "failure_threshold: error\nrule_set: conformance\n",
    )
    .expect("write configuration");
    fs::write(bundle.join("concept.md"), "---\ntitle: Untyped\n---\n").expect("write concept");

    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("check")
        .arg(&bundle)
        .output()
        .expect("run conformance-only rokf check");

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
    assert!(
        stdout.contains("OKF002"),
        "conformance Rule Set should keep Conformance Rule Findings: {stdout}"
    );
    assert!(
        !stdout.contains("OKF101"),
        "conformance Rule Set should filter Quality Rule Findings: {stdout}"
    );
}

#[test]
fn explicit_configuration_reports_parse_errors() {
    let bundle = temp_dir();
    let config = bundle.join("rokf.yml");
    fs::write(bundle.join("index.md"), "# Bundle Index\n").expect("write Root Index File");
    fs::write(&config, "failure_threshold: [").expect("write malformed configuration");

    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("check")
        .arg("--config")
        .arg(&config)
        .arg(&bundle)
        .output()
        .expect("run rokf check with malformed configuration");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("stderr is utf-8");
    assert!(
        stderr.contains("Configuration must be parseable YAML"),
        "explicit Configuration errors should fail clearly: {stderr}"
    );
}

#[test]
fn template_creates_a_concept_document_without_inventing_domain_content() {
    let document = temp_dir().join("customers.md");

    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("template")
        .arg("concept")
        .arg("--type")
        .arg("Warehouse Table")
        .arg(&document)
        .output()
        .expect("run rokf template concept");

    assert!(output.status.success());
    let contents = fs::read_to_string(&document).expect("read concept template");
    assert!(
        contents.contains("type: Warehouse Table"),
        "Concept Type should come from Producer input: {contents}"
    );
    assert!(
        !contents.contains("description:"),
        "template should not invent a Description: {contents}"
    );
}

#[test]
fn template_creates_reserved_file_templates() {
    let dir = temp_dir();
    let index = dir.join("index.md");
    let log = dir.join("log.md");

    let index_output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("template")
        .arg("index")
        .arg(&index)
        .output()
        .expect("run rokf template index");
    let log_output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("template")
        .arg("log")
        .arg(&log)
        .output()
        .expect("run rokf template log");

    assert!(index_output.status.success());
    assert!(log_output.status.success());
    assert_eq!(
        fs::read_to_string(index).expect("read Index File template"),
        "# Index\n"
    );
    assert_eq!(
        fs::read_to_string(log).expect("read Log File template"),
        "# Directory Update Log\n"
    );
}

#[test]
fn template_refuses_to_overwrite_existing_documents() {
    let document = temp_file("customers.md", "existing\n");

    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("template")
        .arg("index")
        .arg(&document)
        .output()
        .expect("run rokf template index on existing document");

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        fs::read_to_string(&document).expect("read existing document"),
        "existing\n"
    );
}

#[test]
fn index_check_reports_missing_and_stale_index_files_without_mutation() {
    let bundle = temp_dir();
    fs::write(bundle.join("index.md"), "# Old\n\n* [Ghost](ghost.md)\n")
        .expect("write stale index");
    fs::write(
        bundle.join("customers.md"),
        "---\ntype: Table\ntitle: Customers\ndescription: Customer records.\n---\n",
    )
    .expect("write concept document");

    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("index")
        .arg("--check")
        .arg(&bundle)
        .output()
        .expect("run rokf index --check");

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
    assert!(
        stdout.contains("OKF500"),
        "index check should report stale Index Maintenance Findings: {stdout}"
    );
    assert!(
        fs::read_to_string(bundle.join("index.md"))
            .expect("read index")
            .contains("Ghost"),
        "index --check should not mutate Index Files"
    );
}

#[test]
fn index_fix_updates_index_files_with_concise_entries() {
    let bundle = temp_dir();
    let nested = bundle.join("tables");
    fs::create_dir_all(&nested).expect("create nested directory");
    fs::write(
        nested.join("customers.md"),
        "---\ntype: Table\ntitle: Customers\ndescription: Customer records.\n---\n",
    )
    .expect("write concept document");
    fs::write(
        nested.join("invoices.md"),
        "---\ntype: Table\ntitle: Invoices\n---\n",
    )
    .expect("write concept document without Description");

    let output = Command::new(env!("CARGO_BIN_EXE_rokf"))
        .arg("index")
        .arg("--fix")
        .arg(&bundle)
        .output()
        .expect("run rokf index --fix");

    assert!(output.status.success());
    let root_index = fs::read_to_string(bundle.join("index.md")).expect("read root index");
    let nested_index = fs::read_to_string(nested.join("index.md")).expect("read nested index");
    assert!(
        root_index.contains("* [Tables](tables/)"),
        "root Index File should include nested directory: {root_index}"
    );
    assert!(
        nested_index.contains("* [Customers](customers.md) - Customer records."),
        "nested Index File should use concise concept entries: {nested_index}"
    );
    assert!(
        nested_index.contains("* [Invoices](invoices.md)"),
        "nested Index File should handle concepts without Descriptions: {nested_index}"
    );
}
