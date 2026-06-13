.PHONY: help quality release-patch release-minor release-major bump-version

QUALITY_CHECKS ?= cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test

define BUMP_VERSION_PY
from pathlib import Path
import os
import re

bump = os.environ["BUMP"]
manifest_path = Path("Cargo.toml")
lockfile_path = Path("Cargo.lock")
manifest = manifest_path.read_text()
match = re.search(r'(?m)^version = "(\d+\.\d+\.\d+)"', manifest)
if not match:
    raise SystemExit("Could not find Cargo package version in Cargo.toml")
major, minor, patch = map(int, match.group(1).split("."))
if bump == "patch":
    patch += 1
elif bump == "minor":
    minor += 1
    patch = 0
elif bump == "major":
    major += 1
    minor = 0
    patch = 0
else:
    raise SystemExit(f"Unsupported release bump: {bump}")
new_version = f"{major}.{minor}.{patch}"
manifest = manifest[:match.start(1)] + new_version + manifest[match.end(1):]
manifest_path.write_text(manifest)
lockfile = lockfile_path.read_text()
lockfile, replacements = re.subn(
    r'(\[\[package\]\]\nname = "rokf"\nversion = ")([^"]+)(")',
    rf'\g<1>{new_version}\3',
    lockfile,
    count=1,
)
if replacements != 1:
    raise SystemExit("Could not find rokf package version in Cargo.lock")
lockfile_path.write_text(lockfile)
print(f"Prepared rokf {new_version}")
endef
export BUMP_VERSION_PY

help:
	@printf '%s\n' 'rokf local release helpers'
	@printf '%s\n' ''
	@printf '%s\n' 'Targets:'
	@printf '%s\n' '  make release-patch  Bump Cargo metadata to the next patch version, then run quality checks.'
	@printf '%s\n' '  make release-minor  Bump Cargo metadata to the next minor version, then run quality checks.'
	@printf '%s\n' '  make release-major  Bump Cargo metadata to the next major version, then run quality checks.'
	@printf '%s\n' ''
	@printf '%s\n' 'These targets prepare a manual release only; they do not publish to crates.io or create GitHub Releases.'

quality:
	$(QUALITY_CHECKS)

release-patch:
	$(MAKE) bump-version BUMP=patch
	$(QUALITY_CHECKS)
	@printf '%s\n' 'Patch release prepared. Review changes, commit, tag, and publish manually when ready.'

release-minor:
	$(MAKE) bump-version BUMP=minor
	$(QUALITY_CHECKS)
	@printf '%s\n' 'Minor release prepared. Review changes, commit, tag, and publish manually when ready.'

release-major:
	$(MAKE) bump-version BUMP=major
	$(QUALITY_CHECKS)
	@printf '%s\n' 'Major release prepared. Review changes, commit, tag, and publish manually when ready.'

bump-version:
	@test -n "$(BUMP)" || (printf '%s\n' 'BUMP is required: patch, minor, or major' >&2; exit 2)
	@BUMP="$(BUMP)" python3 -c "$$BUMP_VERSION_PY"
