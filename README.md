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
anti-patterns and reports them as pass / warning / error with actionable
suggestions.

> [!NOTE]
>
> This project is under active development. Rules and output format may change.

## Why?

ghverify automates SDLC health checks based on the
[SLSA](https://slsa.dev/) framework — review independence, source authenticity,
and build provenance — so teams get fast, consistent feedback without relying
solely on human judgement.

## Controls

All controls are evaluated against an evidence bundle and produce one of four
statuses: **Satisfied**, **Violated**, **Indeterminate**, or **Not Applicable**.
The default `slsa-foundation` profile maps Violated and Indeterminate to
**error / fail**; Satisfied and Not Applicable to **info / pass**.

| Control | Applies to | Description |
|---|---|---|
| `review-independence` | PR, Release | Four-eyes principle: at least one approver must be independent from both the commit author and the PR submitter |
| `source-authenticity` | PR, Release | All source revisions must carry valid, verified cryptographic signatures |
| `build-provenance` | Release | All artifact attestations must carry verified SLSA provenance |

For releases, `review-independence` and `source-authenticity` are evaluated
per commit in the tag range, checking that each commit's associated PR had
independent review and signed commits.

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
      - uses: HikaruEgashira/gh-verify/action/check-pr@main
        with:
          pr-number: ${{ github.event.pull_request.number }}
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
      - uses: HikaruEgashira/gh-verify/action/check-release@main
        with:
          tag: ${{ github.event.release.tag_name }}
```

See [action/check-pr](action/check-pr/README.md) and [action/check-release](action/check-release/README.md) for full input/output details.

## Exit Codes

- `0` — all rules pass (warnings are non-fatal)
- `1` — one or more rules returned an error

## Architecture

Three-crate Rust workspace:

- **gh-verify-core** — Pure verification logic (serde only). No I/O, no unsafe.
- **gh-verify** — CLI binary with GitHub API integration and output formatting.
- **gh-verify-verif** — Creusot formal verification targets. Core predicates with `#[ensures]` specs.

| Extension | Create | Register |
|---|---|---|
| New control | `crates/core/src/controls/<name>.rs` + impl `Control` trait | Add to `controls/mod.rs` `slsa_foundation_controls` |
| New subcommand | Add variant to `Commands` in `main.rs` | clap handles dispatch |
| New output format | `crates/cli/src/output/<name>.rs` | Add case in `output/mod.rs` |
| New API endpoint | `crates/cli/src/github/<name>.rs` | None |

## License

See [LICENSE](LICENSE).
