use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_repo() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "rokf-release-test-{}-{}",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).expect("create temp repo");
    for file in ["Cargo.toml", "Cargo.lock", "Makefile"] {
        fs::copy(file, dir.join(file)).unwrap_or_else(|err| panic!("copy {file}: {err}"));
    }
    dir
}

fn current_package_version() -> (u64, u64, u64) {
    let manifest = fs::read_to_string("Cargo.toml").expect("read Cargo.toml");
    let version = manifest
        .lines()
        .find_map(|line| line.strip_prefix("version = \"")?.strip_suffix('"'))
        .expect("Cargo.toml package version");
    let parts: Vec<_> = version.split('.').collect();

    assert_eq!(parts.len(), 3, "expected semver package version: {version}");
    (
        parts[0].parse().expect("major version"),
        parts[1].parse().expect("minor version"),
        parts[2].parse().expect("patch version"),
    )
}

#[test]
fn release_targets_update_cargo_package_metadata() {
    let (major, minor, patch) = current_package_version();

    for (target, expected_version) in [
        ("release-patch", format!("{major}.{minor}.{}", patch + 1)),
        ("release-minor", format!("{major}.{}.0", minor + 1)),
        ("release-major", format!("{}.0.0", major + 1)),
    ] {
        let repo = temp_repo();

        let output = Command::new("make")
            .arg(target)
            .arg("QUALITY_CHECKS=:")
            .current_dir(&repo)
            .output()
            .unwrap_or_else(|err| panic!("run make {target}: {err}"));

        assert!(
            output.status.success(),
            "{target} should succeed; stdout: {}; stderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let manifest = fs::read_to_string(repo.join("Cargo.toml")).expect("read updated manifest");
        assert!(
            manifest.contains(&format!("version = \"{expected_version}\"")),
            "{target} should bump Cargo.toml version: {manifest}"
        );

        let lockfile = fs::read_to_string(repo.join("Cargo.lock")).expect("read updated lockfile");
        assert!(
            lockfile.contains(&format!(
                "name = \"rokf\"\nversion = \"{expected_version}\""
            )),
            "{target} should keep Cargo.lock package metadata consistent: {lockfile}"
        );
    }
}

#[test]
fn release_targets_run_quality_checks_but_do_not_publish() {
    let output = Command::new("make")
        .arg("--dry-run")
        .arg("release-minor")
        .output()
        .expect("dry-run make release-minor");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("dry-run output is utf-8");

    assert!(
        stdout.contains("cargo fmt --check"),
        "should check formatting: {stdout}"
    );
    assert!(
        stdout.contains("cargo clippy --all-targets -- -D warnings"),
        "should run clippy: {stdout}"
    );
    assert!(stdout.contains("cargo test"), "should run tests: {stdout}");
    assert!(
        !stdout.contains("cargo publish") && !stdout.contains("gh release"),
        "manual release preparation should not publish or create a GitHub Release: {stdout}"
    );
}

#[test]
fn makefile_documents_release_commands() {
    let output = Command::new("make")
        .arg("help")
        .output()
        .expect("run make help");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("help output is utf-8");
    for target in ["release-patch", "release-minor", "release-major"] {
        assert!(
            stdout.contains(target),
            "Makefile help should document {target}: {stdout}"
        );
    }
}
