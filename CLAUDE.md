# ghverify - GitHub SDLC Verifier

GitHub SDLC health checker. Runs as a `gh` CLI extension, built in Rust.
Core verification logic lives in [libverify](https://github.com/HikaruEgashira/libverify).
Formal verification with Creusot + SMT solvers in libverify-verif and crates/verif.

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

gh-verify is a thin GitHub-specific shell over libverify.

### Dependencies

- `libverify-core` — evidence model, Control trait, 20 built-in controls, assessment engine, SLSA v1.2 mapping
- `libverify-policy` — OPA Rego policy engine with 5 presets (default, oss, aiops, soc1, soc2)
- `libverify-output` — SARIF/JSON output formatters

### gh-verify (crates/cli/)

I/O layer. Delegates all judgments to libverify via the control/evidence assessment path.

| Change | Where | Registration |
|---|---|---|
| New control | libverify repo: `crates/core/src/controls/<name>.rs` | See libverify CLAUDE.md |
| New subcommand | Add variant to `Commands` enum in `main.rs` | clap handles dispatch |
| New output format | `crates/cli/src/output/<name>.rs` | Add case in `output/mod.rs` |
| New API endpoint | `crates/cli/src/github/<name>.rs` | Register in `github/mod.rs` |
| New adapter | `crates/cli/src/adapters/<name>.rs` | None |

### gh-verify-verif (crates/verif/)

Creusot verification targets. Core predicates with `#[ensures]` specs.
Runtime implementations in libverify-core must match these verified predicates.

## Naming

- Control ID: kebab-case string (`"review-independence"`)
- Built-in constant: `libverify_core::control::builtin::REVIEW_INDEPENDENCE`
- File name: snake_case (`review_independence.rs`)

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
