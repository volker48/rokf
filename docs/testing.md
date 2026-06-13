# Testing layout

Production code lives under `src/`. Behavior tests live under `tests/` and mirror the production source concept they exercise.

For example, CLI behavior defined in `src/cli.rs` is covered by `tests/cli.rs`. Put shared command runners, fixtures, or other test-only helpers under `tests/` rather than `src/` so production modules stay focused on rokf behavior.

Prefer behavior-focused integration tests through public interfaces, such as invoking the `rokf` binary for CLI affordances.
