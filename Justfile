# rokf dev helpers — https://github.com/casey/just
#
# Install:   brew install just
# Quickstart: just check rex

default:
    @just --list

# Compile the debug binary (required by other recipes)
build:
    cargo build

# Fuzzy-match a demo Bundle Root or Concept Document by substring and run `rokf check` against it.
# Existing paths are used directly, so `just check demos` checks the demo hierarchy.
# Usage: just check demos
#        just check star-wars
#        just check rex
#        just check phase-ii
check term: build
    #!/usr/bin/env bash
    set -euo pipefail
    term="{{term}}"
    if [ -e "$term" ]; then
        target="$term"
    else
        dir_matches=$(find demos -type d | grep -i "$term" || true)
        dir_count=$(echo "$dir_matches" | sed '/^$/d' | wc -l | tr -d ' ')
        if [ "$dir_count" -eq 1 ]; then
            target="$dir_matches"
        elif [ "$dir_count" -gt 1 ]; then
            echo "Multiple demo Bundle Root matches for '$term':" >&2
            echo "$dir_matches" >&2
            exit 1
        else
            file_matches=$(find demos -type f -name "*.md" | grep -i "$term" || true)
            file_count=$(echo "$file_matches" | sed '/^$/d' | wc -l | tr -d ' ')
            if [ "$file_count" -eq 0 ]; then
                echo "No demo Bundle Root or Concept Document matching '$term' found" >&2
                exit 1
            elif [ "$file_count" -gt 1 ]; then
                echo "Multiple demo Concept Document matches for '$term':" >&2
                echo "$file_matches" >&2
                exit 1
            fi
            target="$file_matches"
        fi
    fi
    echo "📄 $target"
    ./target/debug/rokf check "$target"

# Fuzzy-match a demo Bundle Root or Concept Document by substring and run `rokf format` against it.
# Existing paths are used directly, so `just format demos` formats the demo hierarchy.
# Usage: just format demos
#        just format star-wars
#        just format rex
#        just format phase-ii
format term: build
    #!/usr/bin/env bash
    set -euo pipefail
    term="{{term}}"
    if [ -e "$term" ]; then
        target="$term"
    else
        dir_matches=$(find demos -type d | grep -i "$term" || true)
        dir_count=$(echo "$dir_matches" | sed '/^$/d' | wc -l | tr -d ' ')
        if [ "$dir_count" -eq 1 ]; then
            target="$dir_matches"
        elif [ "$dir_count" -gt 1 ]; then
            echo "Multiple demo Bundle Root matches for '$term':" >&2
            echo "$dir_matches" >&2
            exit 1
        else
            file_matches=$(find demos -type f -name "*.md" | grep -i "$term" || true)
            file_count=$(echo "$file_matches" | sed '/^$/d' | wc -l | tr -d ' ')
            if [ "$file_count" -eq 0 ]; then
                echo "No demo Bundle Root or Concept Document matching '$term' found" >&2
                exit 1
            elif [ "$file_count" -gt 1 ]; then
                echo "Multiple demo Concept Document matches for '$term':" >&2
                echo "$file_matches" >&2
                exit 1
            fi
            target="$file_matches"
        fi
    fi
    echo "📄 $target"
    ./target/debug/rokf format "$target"

# Run `rokf check` against each top-level demo Bundle Root and print a pass/fail summary.
check-all: build
    #!/usr/bin/env bash
    set -euo pipefail
    find demos -mindepth 1 -maxdepth 1 -type d | sort | while read -r bundle; do
        if ./target/debug/rokf check "$bundle" > /dev/null 2>&1; then
            echo "  OK  $bundle"
        else
            echo "FAIL  $bundle"
        fi
    done

# Watch source files and re-check a demo Bundle Root or Concept Document on every Rust change.
# Requires: cargo install cargo-watch
# Usage: just watch demos
#        just watch rex
watch term:
    #!/usr/bin/env bash
    set -euo pipefail
    term="{{term}}"
    if [ -e "$term" ]; then
        target="$term"
    else
        dir_matches=$(find demos -type d | grep -i "$term" || true)
        dir_count=$(echo "$dir_matches" | sed '/^$/d' | wc -l | tr -d ' ')
        if [ "$dir_count" -eq 1 ]; then
            target="$dir_matches"
        elif [ "$dir_count" -gt 1 ]; then
            echo "Multiple demo Bundle Root matches for '$term':" >&2
            echo "$dir_matches" >&2
            exit 1
        else
            file_matches=$(find demos -type f -name "*.md" | grep -i "$term" || true)
            file_count=$(echo "$file_matches" | sed '/^$/d' | wc -l | tr -d ' ')
            if [ "$file_count" -eq 0 ]; then
                echo "No demo Bundle Root or Concept Document matching '$term' found" >&2
                exit 1
            elif [ "$file_count" -gt 1 ]; then
                echo "Multiple demo Concept Document matches for '$term':" >&2
                echo "$file_matches" >&2
                exit 1
            fi
            target="$file_matches"
        fi
    fi
    echo "Watching sources — checking $target on change..."
    cargo watch -x "run -- check $target"
