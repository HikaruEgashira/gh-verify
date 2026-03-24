# gh-verify — GitHub SDLC Verifier

Thin `gh` CLI extension for SDLC health checking. All verification logic lives in [libverify](https://github.com/HikaruEgashira/libverify).

## Commands

```bash
devenv tasks run ghverify:build    # Release build
devenv tasks run ghverify:test     # Unit tests
devenv tasks run ghverify:bench    # Benchmarks (uses GitHub API)
devenv tasks run ghverify:dist     # Build release binary for distribution
devenv tasks run ghverify:fmt      # Format + clippy lint
```

## Architecture

gh-verify is a thin CLI shell (~300 LOC). All domain logic is in libverify.

### Dependencies

- `libverify-github` — GitHub API client, evidence adapter, verification orchestration
- `libverify-core` — evidence model, controls, assessment engine
- `libverify-output` — SARIF/JSON output formatters
- `libverify-policy` — OPA/Rego policy engine, built-in presets

### Where to make changes

| Change | Where |
|---|---|
| New control | libverify: `crates/core/src/controls/<name>.rs` |
| New GitHub API | libverify: `crates/github/src/` |
| New subcommand | `main.rs` — add variant to `Commands` enum |
| New output format | `output/mod.rs` — add case |
| New policy preset | libverify: `crates/policy/src/<name>.rego` + register in `lib.rs` |
| Formal verification | libverify: `crates/verif/` |
