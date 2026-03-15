<h1 align="center">ghverify</h1>

<p align="center">
  A SLSA-based SDLC verifier for GitHub pull requests.
</p>

<p align="center">
  <a href="HACKING.md">Hacking</a> · <a href="action/check-pr/README.md">GitHub Action</a> · <a href="benchmarks/README.md">Benchmarks</a>
</p>

---

**ghverify** checks whether a pull request follows healthy software development
lifecycle practices. It runs as a `gh` CLI extension, built in Rust with
core verification logic formally specified via [Creusot](https://github.com/creusot-rs/creusot).

The tool analyzes PR diffs and metadata to detect anti-patterns — such as
changes that span too many unrelated domains — and reports them as
pass / warning / error with actionable suggestions.

> [!NOTE]
>
> This project is under active development. Rules and output format may change.

## Why?

Large, unfocused pull requests are hard to review, easy to mis-merge, and
a leading cause of subtle regressions. Automated scope checks catch these
problems before a reviewer has to.

ghverify enforces this at the PR level so teams get fast, consistent feedback
without relying solely on human judgement.

## Rules

| Rule | Severity | Description |
|---|---|---|
| `detect-unscoped-change` | warning / error | Flags PRs that touch multiple unrelated domains |
| `detect-missing-test` | warning | Warns when source changes have no matching test updates |
| `verify-release-integrity` | error | Checks commit signatures, mutual approval, PR coverage |

Run `gh verify pr list-rules` to see all registered rules.

## Usage

### CLI

```bash
# Verify a PR
gh verify pr 123 --repo owner/repo

# Verify a PR and disable missing-test detection
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

Two-crate Rust workspace:

- **gh-verify-core** — Pure verification logic with Creusot formal specifications. No I/O, no unsafe.
- **gh-verify** — CLI binary with GitHub API integration, tree-sitter analysis, and output formatting.

| Extension | Create | Register |
|---|---|---|
| New rule | `crates/cli/src/rules/<name>.rs` | Add to `engine.rs` |
| New subcommand | Add variant to `Commands` in `main.rs` | clap handles dispatch |
| New output format | `crates/cli/src/output/<name>.rs` | Add case in `output/mod.rs` |
| New API endpoint | `crates/cli/src/github/<name>.rs` | None |

## License

See [LICENSE](LICENSE).
