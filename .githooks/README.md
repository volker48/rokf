# Git hooks

This repository keeps versioned Git hooks in `.githooks/`.

Enable them in a clone with:

```sh
git config core.hooksPath .githooks
```

The pre-commit hook runs the same Rust quality gates as CI:

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-targets --all-features`
