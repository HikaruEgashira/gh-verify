<h1 align="center">ghverify</h1>

<p align="center">
  A SLSA-based SDLC verifier for GitHub repositories.
</p>

<p align="center">
  <a href="https://scorecard.dev/viewer/?uri=github.com/HikaruEgashira/gh-verify"><img src="https://api.scorecard.dev/projects/github.com/HikaruEgashira/gh-verify/badge" alt="OpenSSF Scorecard"></a>
</p>

<p align="center">
  <a href="HACKING.md">Hacking</a> · <a href="action/check-pr/README.md">GitHub Action</a> · <a href="benchmarks/README.md">Benchmarks</a>
</p>

---

**ghverify** verifies that pull requests and releases follow healthy software
development lifecycle practices. It runs as a `gh` CLI extension, built in
Rust with core verification logic formally specified via
[Creusot](https://github.com/creusot-rs/creusot).

The tool analyzes diffs, metadata, and release artifacts to detect
anti-patterns and reports them as pass / warning / error with actionable
suggestions.

> [!NOTE]
>
> This project is under active development. Rules and output format may change.

## Why?

ghverify automates SDLC health checks — PR scope, test coverage, approval
integrity, commit hygiene, release provenance — so teams get fast, consistent
feedback without relying solely on human judgement.

## Rules

### PR rules

| Rule | Severity | Description |
|---|---|---|
| `detect-unscoped-change` | warning / error | Flags PRs that touch multiple unrelated domains (tree-sitter call graph analysis) |
| `detect-missing-test` | warning / error | Warns when source changes lack test coverage (LCOV) or matching test file updates (heuristic) |
| `detect-stale-approval` | warning / error | Detects commits pushed after the last approval (four-eyes principle bypass) |
| `verify-issue-linkage` | error | Requires PR body to reference an issue or ticket (GitHub, Jira, URL) |
| `verify-pr-size` | warning / error | Flags oversized PRs by line count or file count |
| `verify-conventional-commit` | warning / error | Checks PR title / commit messages against Conventional Commits spec |

### Release rules

| Rule | Severity | Description |
|---|---|---|
| `verify-release-integrity` | error | Checks commit signatures, mutual approval, PR coverage (SLSA) |
| `verify-branch-protection` | warning / error | Verifies PRs target protected branches with review activity |

Run `gh verify pr list-rules` to see all registered rules.

## Usage

### CLI

```bash
# Verify a PR
gh verify pr 123 --repo owner/repo

# Verify with LCOV coverage report
gh verify pr 123 --repo owner/repo --coverage target/llvm-cov/lcov.info

# Disable missing-test detection
gh verify pr 123 --repo owner/repo --no-detect-missing-test

# JSON output
gh verify pr 123 --repo owner/repo --format json

# List available rules
gh verify pr list-rules

# Verify a release
gh verify release v1.0.0 --repo owner/repo
gh verify release v0.9.0..v1.0.0 --repo owner/repo
```

### GitHub Action

Add to `.github/workflows/verify.yml`:

```yaml
on:
  pull_request:
    types: [opened, synchronize]

jobs:
  verify:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      pull-requests: read
    steps:
      - uses: HikaruEgashira/gh-verify/action/check-pr@main
        with:
          pr-number: ${{ github.event.pull_request.number }}
```

See [action/check-pr](action/check-pr/README.md) for full input/output details.

## Exit Codes

- `0` — all rules pass (warnings are non-fatal)
- `1` — one or more rules returned an error

## Architecture

Three-crate Rust workspace:

- **gh-verify-core** — Pure verification logic (serde only). No I/O, no unsafe.
- **gh-verify** — CLI binary with GitHub API integration, tree-sitter analysis, and output formatting.
- **gh-verify-verif** — Creusot formal verification targets. Core predicates with `#[ensures]` specs.

| Extension | Create | Register |
|---|---|---|
| New rule | `crates/cli/src/rules/<name>.rs` | Add to `engine.rs` |
| New subcommand | Add variant to `Commands` in `main.rs` | clap handles dispatch |
| New output format | `crates/cli/src/output/<name>.rs` | Add case in `output/mod.rs` |
| New API endpoint | `crates/cli/src/github/<name>.rs` | None |

## License

See [LICENSE](LICENSE).
