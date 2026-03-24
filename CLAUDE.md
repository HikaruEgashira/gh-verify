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

### Source files

| File | Role |
|---|---|
| `crates/cli/src/main.rs` | clap CLI definition + dispatch |
| `crates/cli/src/output/mod.rs` | Format dispatch (human local, json/sarif via libverify-output) |
| `crates/cli/src/output/human.rs` | Colored terminal output |
| `crates/cli/src/bin/bench.rs` | Benchmark binary |

### Where to make changes

| Change | Where |
|---|---|
| New control | libverify: `crates/core/src/controls/<name>.rs` |
| New GitHub API | libverify: `crates/github/src/` |
| New subcommand | `main.rs` — add variant to `Commands` enum |
| New output format | `output/mod.rs` — add case |
| New policy preset | libverify: `crates/policy/src/<name>.rego` + register in `lib.rs` |
| Formal verification | libverify: `crates/verif/` |

## Policy

All verification behavior is controlled by `--policy`. No separate `--slsa_level` flag.

Available presets: `default`, `oss`, `aiops`, `soc1`, `soc2`, `slsa-l1`, `slsa-l2`, `slsa-l3`, `slsa-l4`.
Custom `.rego` file paths are also accepted.

Default policy is `default` (all controls strict). Use `--policy slsa-l1` for SLSA L1 behavior.

## PR Template

```markdown
## What
## Why
## How
## Verification
- [ ] `cargo test` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `--format json` output is valid JSON
```
