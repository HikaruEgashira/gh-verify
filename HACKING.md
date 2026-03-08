# Hacking on ghverify

## Setup

```bash
# devenv (recommended: provides rust / gh / jq automatically)
direnv allow   # or: devenv shell

# or: use mise/rustup
cargo build
```

## Tasks

All development tasks are managed via devenv. Run with `devenv tasks run <task>`.

```bash
devenv tasks run ghverify:build      # Release build
devenv tasks run ghverify:test       # Unit + integration tests (no network required)
devenv tasks run ghverify:bench      # Benchmarks (uses GitHub API)
devenv tasks run ghverify:dist       # Build release binary
```

## Adding a Rule

1. Create `crates/core/src/<logic>.rs` if the rule needs pure judgment logic (formally verifiable)
2. Create `crates/cli/src/rules/<name>.rs` — implement the `Rule` trait
3. Add `Box::new(YourRule)` to the `run_all` Vec in `crates/cli/src/rules/engine.rs`

## Adding a Subcommand

1. Add a variant to the `Commands` enum in `crates/cli/src/main.rs`
2. Handle it in the `run()` function

## Formal Verification

The `gh-verify-core` crate contains pure verification logic with Creusot specifications:

```bash
# Build with contracts feature (requires Creusot toolchain)
cargo build -p gh-verify-core --features contracts

# Run standard tests (always works)
cargo test -p gh-verify-core
```

## Release

```bash
git tag v0.2.0
git push origin v0.2.0
# → GitHub Actions builds and releases
```
