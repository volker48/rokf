## Agent skills

### Issue tracker

Issues are tracked in GitHub Issues using the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Triage uses the default five-label vocabulary. See `docs/agents/triage-labels.md`.

### Domain docs

This repo uses a single-context domain documentation layout. See `docs/agents/domain.md`.

### Dev loop (Justfile)

`just check <term>` fuzzy-matches a demo file and runs `rokf check` against it. When adding a new subcommand (e.g. `format`), copy the `check` recipe, rename it, and swap the subcommand — keep the same fuzzy-match pattern. See README for usage.
