# ghverify - GitHub SDLC Verifier

SLSA-based GitHub SDLC health checker. Runs as a `gh` CLI extension, built in Rust.
Core verification logic is formally specified with Creusot annotations.

## Commands

```bash
cargo build                                    # Debug build
cargo build --release -p gh-verify             # Release build
cargo test --workspace                         # Run all tests
./target/release/gh-verify pr 123 --repo o/r   # PR verify
./target/release/gh-verify pr 123 --format json # JSON output
./target/release/gh-verify pr list-rules       # List rules
```

## Architecture

Two-crate workspace: `gh-verify-core` (pure, formally verifiable) + `gh-verify` (CLI with I/O).

### gh-verify-core (crates/core/)

Pure verification logic. No I/O, no unsafe, no external deps except serde.
Creusot formal specs via `#[cfg(feature = "contracts")]`.

| Module | Purpose | Creusot specs |
|--------|---------|---------------|
| `verdict.rs` | Severity, RuleResult types | Ordering invariant |
| `integrity.rs` | SLSA release checks (signatures, approval, coverage) | Biconditional pass/fail specs |
| `scope.rs` | PR scope classification | Exhaustive classify_scope postconditions |
| `union_find.rs` | Disjoint set union for call graph | Structural invariants, idempotency, transitivity |

### gh-verify (crates/cli/)

I/O layer. Delegates all judgments to core.

| Change | File to create | Registration |
|---|---|---|
| New rule | `crates/cli/src/rules/<name>.rs` + impl `Rule` trait | Add to `engine.rs` `run_all` Vec |
| New subcommand | Add variant to `Commands` enum in `main.rs` | clap handles dispatch |
| New output format | `crates/cli/src/output/<name>.rs` | Add case in `output/mod.rs` |
| New API endpoint | `crates/cli/src/github/<name>.rs` | None |

## Naming

- Rule ID: kebab-case (`detect-unscoped-change`)
- File name: snake_case (`detect_unscoped_change.rs`)
- Crate name: kebab-case (`gh-verify-core`)

## Exit Codes

- `0`: all rules pass / warnings only
- `1`: one or more rules returned error

## Dependencies

- tree-sitter (core, go, python, typescript): via crates.io
- reqwest: HTTP client
- clap: CLI parsing
- serde/serde_json: serialization
- creusot-std: formal verification specs (optional, feature-gated)

## PR Template

```markdown
## What
## Why
## How
## Verification
- [ ] `cargo test --workspace` passes
- [ ] Existing rules still work
- [ ] For new rules: verified pass/warning/error cases
- [ ] `--format json` output is valid JSON
```
