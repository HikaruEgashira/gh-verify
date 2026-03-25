# Hacking on ghverify

## Setup

```bash
# devenv (recommended: provides rust / gh / jq automatically)
direnv allow   # or: devenv shell
```

## Development

All commands are devenv tasks:

```bash
devenv tasks run ghverify:build          # Release build
devenv tasks run ghverify:test           # Unit + integration tests (no network)
devenv tasks run ghverify:bench          # Benchmarks (uses GitHub API)
devenv tasks run ghverify:dist           # Build release binary
devenv tasks run ghverify:fmt            # Format + clippy lint
```

## Adding a Control

### SLSA control

1. Add `ControlId` variant in `crates/core/src/control.rs` (with `as_str` + `from_str`)
2. Map it in `crates/core/src/slsa.rs` (`control_slsa_mapping` + `ALL_SLSA_CONTROLS`)
3. Add integrity predicate in `crates/core/src/integrity.rs`
4. Create `crates/core/src/controls/<name>.rs` — implement `Control` trait, delegate to integrity predicate
5. Add to `crates/core/src/controls/mod.rs` (`instantiate()` match arm)

### Dev quality control

1. Add `ControlId` variant in `crates/core/src/control.rs`
2. Create `crates/core/src/controls/<name>.rs` — implement `Control` trait
3. Add to `crates/core/src/controls/mod.rs` (`instantiate()` + `compliance_controls()`)

## Adding a Subcommand

1. Add a variant to the `Commands` enum in `crates/cli/src/main.rs`
2. Handle it in the `run()` function

## Release

```bash
git tag v0.2.0
git push origin v0.2.0
# → GitHub Actions builds and releases
```
