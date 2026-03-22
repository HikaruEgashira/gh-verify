<h1 align="center">ghverify</h1>

<p align="center">
  A SLSA-based SDLC verifier for GitHub repositories.
</p>

<p align="center">
  <a href="https://scorecard.dev/viewer/?uri=github.com/HikaruEgashira/gh-verify"><img src="https://api.scorecard.dev/projects/github.com/HikaruEgashira/gh-verify/badge" alt="OpenSSF Scorecard"></a>
</p>

<p align="center">
  <a href="HACKING.md">Hacking</a> · <a href="benchmarks/README.md">Benchmarks</a>
</p>

---

**ghverify** verifies that pull requests and releases follow healthy software
development lifecycle practices. It runs as a `gh` CLI extension, built in
Rust with core verification logic formally specified via
[Creusot](https://github.com/creusot-rs/creusot).

The tool analyzes diffs, metadata, and release artifacts to detect
anti-patterns and reports them as pass / review / fail with actionable
suggestions.

> [!NOTE]
>
> This project is under active development. Controls and output format may change.

## Why?

ghverify automates SDLC health checks based on the
[SLSA](https://slsa.dev/) framework — review independence, source authenticity,
and build provenance — so teams get fast, consistent feedback without relying
solely on human judgement.

## Controls

All controls are evaluated against an evidence bundle and produce one of four
statuses: **Satisfied**, **Violated**, **Indeterminate**, or **Not Applicable**.
The default profile maps Violated and Indeterminate to **fail**; Satisfied and
Not Applicable to **pass**.

### SLSA Foundation

| Control | Applies to | Description |
|---|---|---|
| `review-independence` | PR, Release | Four-eyes principle: at least one approver must be independent from both the commit author and the PR submitter |
| `source-authenticity` | PR, Release | All source revisions must carry valid, verified cryptographic signatures |
| `build-provenance` | Release | All artifact attestations must carry verified SLSA provenance |
| `required-status-checks` | PR | All CI check runs on the PR HEAD commit must pass |

For releases, `review-independence` and `source-authenticity` are evaluated
per commit in the tag range, checking that each commit's associated PR had
independent review and signed commits.

### Development Quality

| Control | Applies to | Description |
|---|---|---|
| `pr-size` | PR | PR size is within acceptable limits (lines changed, files changed) |
| `test-coverage` | PR | Source file changes include corresponding test updates |
| `scoped-change` | PR | PR changes are well-scoped as a single logical unit |
| `issue-linkage` | PR | PR references at least one issue or ticket |

## Usage

### CLI

```bash
# Verify a pull request
gh verify pr 123 --repo owner/repo

# JSON output
gh verify pr 123 --repo owner/repo --format json

# Verify a release (auto-detect previous tag)
gh verify release v1.0.0 --repo owner/repo

# Verify a release between two tags
gh verify release v0.9.0..v1.0.0 --repo owner/repo

# Use a custom OPA policy for gate decisions
gh verify pr 123 --repo owner/repo --policy policy.rego
```

### GitHub Action

**PR verification** — add to `.github/workflows/verify.yml`:

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
      - uses: HikaruEgashira/gh-verify@v0.4
        with:
          command: pr
          argument: ${{ github.event.pull_request.number }}
```

**Release verification** — add to `.github/workflows/verify-release.yml`:

```yaml
on:
  release:
    types: [published]

jobs:
  verify:
    runs-on: ubuntu-latest
    permissions:
      contents: read
    steps:
      - uses: HikaruEgashira/gh-verify@v0.4
        with:
          command: release
          argument: ${{ github.event.release.tag_name }}
```

See [action.yml](action.yml) for full input/output details.

## Exit Codes

- `0` — no control received a **fail** gate decision
- `1` — one or more controls received a **fail** gate decision

## Architecture

Three-crate Rust workspace:

- **gh-verify-core** — Pure verification logic (serde only). No I/O, no unsafe.
- **gh-verify** — CLI binary with GitHub API integration and output formatting.
- **gh-verify-verif** — Creusot formal verification targets. Core predicates with `#[ensures]` specs.

| Extension | Create | Register |
|---|---|---|
| New control | `crates/core/src/controls/<name>.rs` + impl `Control` trait | Add to `controls/mod.rs` |
| New subcommand | Add variant to `Commands` in `main.rs` | clap handles dispatch |
| New output format | `crates/cli/src/output/<name>.rs` | Add case in `output/mod.rs` |
| New API endpoint | `crates/cli/src/github/<name>.rs` | None |

## License

See [LICENSE](LICENSE).
