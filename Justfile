# rokf dev helpers — https://github.com/casey/just
#
# Install:   brew install just
# Quickstart: just check rex

default:
    @just --list

# Compile the debug binary (required by other recipes)
build:
    cargo build

# Fuzzy-match a demo file by substring and run `rokf check` against it.
# Usage: just check rex
#        just check phase-ii
check term: build
    #!/usr/bin/env bash
    set -euo pipefail
    matches=$(find demos -type f -name "*.md" | grep -i "{{term}}" || true)
    if [ -z "$matches" ]; then
        echo "No demo file matching '{{term}}' found" >&2
        exit 1
    fi
    count=$(echo "$matches" | wc -l | tr -d ' ')
    if [ "$count" -gt 1 ]; then
        echo "Multiple matches for '{{term}}':" >&2
        echo "$matches" >&2
        exit 1
    fi
    echo "📄 $matches"
    ./target/debug/rokf check "$matches"

# Run `rokf check` against every demo file and print a pass/fail summary.
check-all: build
    #!/usr/bin/env bash
    set -euo pipefail
    find demos -type f -name "*.md" | sort | while read -r file; do
        if ./target/debug/rokf check "$file" > /dev/null 2>&1; then
            echo "  OK  $file"
        else
            echo "FAIL  $file"
        fi
    done

# Watch source files and re-check a demo file on every Rust change.
# Requires: cargo install cargo-watch
# Usage: just watch rex
watch term:
    #!/usr/bin/env bash
    set -euo pipefail
    file=$(find demos -type f -name "*.md" | grep -i "{{term}}" | head -1)
    if [ -z "$file" ]; then
        echo "No demo file matching '{{term}}' found" >&2
        exit 1
    fi
    echo "Watching sources — checking $file on change..."
    cargo watch -x "run -- check $file"
