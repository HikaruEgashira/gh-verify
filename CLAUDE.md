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
./scripts/verify.sh                      # Creusot formal verification (all)
./scripts/verify.sh <predicate_name>     # Creusot verify single predicate
```

## Architecture

Three-crate workspace:

- `gh-verify-core` — pure runtime logic (serde only)
- `gh-verify` — CLI with I/O (reqwest, clap, tree-sitter)
- `gh-verify-verif` — Creusot verification targets (creusot-std only)

### gh-verify-core (crates/core/)

Pure verification logic. No I/O, no unsafe.

| Module | Purpose |
|--------|---------|
| `verdict.rs` | Severity enum, RuleResult type |
| `integrity.rs` | SLSA release checks (signatures, mutual approval, PR coverage) |
| `scope.rs` | PR scope classification by connected components |
| `union_find.rs` | Disjoint set union for call graph connectivity |
| `coverage.rs` | LCOV parser, patch line extraction, coverage analysis |
| `test_coverage.rs` | Test file pair heuristics (naming convention fallback) |
| `approval.rs` | Stale approval detection (timestamp comparison) |
| `branch_protection.rs` | Branch protection compliance checks |
| `conventional.rs` | Conventional Commits format validation |
| `linkage.rs` | Issue/ticket reference extraction from PR body |
| `size.rs` | PR size classification by line/file count |

### gh-verify-verif (crates/verif/)

Creusot verification targets. Core predicates with `#[ensures]` specs
in a crate free of Creusot-unsupported constructs (`format!`, `String`, `Vec`).
Runtime implementations in `gh-verify-core` must match these verified predicates.

### gh-verify (crates/cli/)

I/O layer. Delegates all judgments to core via the control/evidence assessment path.

| Module | Purpose |
|--------|---------|
| `config.rs` | GH_TOKEN / GH_REPO env resolution |
| `github/client.rs` | HTTP client with User-Agent |
| `github/pr_api.rs` | PR files / metadata / reviews / commits fetch |
| `github/release_api.rs` | Tag comparison, commit-PR association, reviews |
| `adapters/github.rs` | GitHub API → `EvidenceBundle` mapping |
| `output/` | human / json formatters for `AssessmentReport` |
| `util/symbol_extractor.rs` | tree-sitter symbol extraction |

| Change | Where | Registration |
|---|---|---|
| New control | `crates/core/src/controls/<name>.rs` + impl `Control` trait | Add to `controls/mod.rs` `slsa_foundation_controls` |
| New subcommand | Add variant to `Commands` enum in `main.rs` | clap handles dispatch |
| New output format | `crates/cli/src/output/<name>.rs` | Add case in `output/mod.rs` |
| New API endpoint | `crates/cli/src/github/<name>.rs` | None |
| New adapter | `crates/cli/src/adapters/<name>.rs` | None |

## Naming

- Control ID: PascalCase enum variant (`ReviewIndependence`)
- File name: snake_case (`review_independence.rs`)
- Crate name: kebab-case (`gh-verify-core`)

## Reusable Actions

Two composite actions under `action/` for use in GitHub Actions workflows:

| Action | Trigger | Purpose |
|---|---|---|
| `action/check-pr` | `pull_request` + `pull_request_review` | Run SDLC checks on a PR |
| `action/check-release` | `push: tags: v*` (via `release.yml`) | Gate release build on SDLC checks |

Usage from other repositories:

```yaml
- uses: HikaruEgashira/gh-verify/action/check-pr@v0.4.0
  with:
    pr-number: ${{ github.event.pull_request.number }}
```

## Exit Codes

- `0`: all control outcomes are Pass/Review
- `1`: one or more control outcomes are Fail

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
