# ghverify - GitHub SDLC Verifier

SLSA-based GitHub SDLC health checker. Runs as a `gh` CLI extension, built in Rust.
Core verification logic is formally proven with Creusot + SMT solvers.

## Commands

All development commands are devenv tasks. Run with `devenv tasks run <task>`.

```bash
devenv tasks run ghverify:build          # Release build
devenv tasks run ghverify:test           # Unit + integration tests (no network)
devenv tasks run ghverify:bench          # Benchmarks (uses GitHub API)
devenv tasks run ghverify:dist           # Build release binary for distribution
devenv tasks run ghverify:fmt            # Format + clippy lint
devenv tasks run ghverify:docs            # Generate rule docs from tests/specs → site/
devenv tasks run ghverify:verify          # Creusot formal verification (all)
devenv tasks run ghverify:verify-one <name>  # Creusot verify single predicate
```

## Architecture

Three-crate workspace:

- `gh-verify-core` — pure runtime logic (serde only)
- `gh-verify` — CLI with I/O (reqwest, clap)
- `gh-verify-verif` — Creusot verification targets (creusot-std only)

### gh-verify-core (crates/core/)

Pure verification logic. No I/O, no unsafe. Entry point: `assess_with_slsa_foundation`.

### gh-verify-verif (crates/verif/)

Creusot verification targets. Core predicates with `#[ensures]` specs
in a crate free of Creusot-unsupported constructs (`format!`, `String`, `Vec`).
Runtime implementations in `gh-verify-core` must match these verified predicates.

### gh-verify (crates/cli/)

I/O layer. Delegates all judgments to core via the control/evidence assessment path.

| Change | Where | Registration |
|---|---|---|
| New control | `crates/core/src/controls/<name>.rs` + impl `Control` trait | Add to `controls/mod.rs` |
| New subcommand | Add variant to `Commands` enum in `main.rs` | clap handles dispatch |
| New output format | `crates/cli/src/output/<name>.rs` | Add case in `output/mod.rs` |
| New API endpoint | `crates/cli/src/github/<name>.rs` | None |
| New adapter | `crates/cli/src/adapters/<name>.rs` | None |

## Naming

- Control ID: PascalCase enum variant (`ReviewIndependence`)
- File name: snake_case (`review_independence.rs`)
- Crate name: kebab-case (`gh-verify-core`)

## PR Template

```markdown
## What
## Why
## How
## Verification
- [ ] `devenv tasks run ghverify:test` passes
- [ ] Existing controls still work
- [ ] For new controls: verified pass/fail/indeterminate cases
- [ ] `--format json` output is valid JSON
- [ ] `devenv tasks run ghverify:verify` passes for affected predicates
```
