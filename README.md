<h1 align="center">gh-verify</h1>

<p align="center">
  A SLSA-based SDLC verifier for GitHub repositories.
</p>

<p align="center">
  <a href="HACKING.md">Hacking</a> · <a href="benchmarks/README.md">Benchmarks</a> · <a href="docs/adr/">ADRs</a>
</p>

---

gh-verify verifies that pull requests and releases follow
[SLSA v1.2](https://slsa.dev/), [NIST SSDF (SP 800-218)](https://csrc.nist.gov/pubs/sp/800/218/final),
and [OpenSSF Scorecard](https://scorecard.dev/) supply chain security practices.
It runs as a `gh` CLI extension, built in Rust with core verification
logic formally proven via [Creusot](https://github.com/creusot-rs/creusot).

Each control evaluates evidence and produces a verdict:
Satisfied, Violated, Indeterminate, or Not Applicable.
The profile maps these to gate decisions — pass, review, or fail.

> [!WARNING]
>
> This project is under active development. Controls and output format may change.

## Usage

### CLI

```bash
gh verify pr 123 --repo owner/repo
gh verify release v1.0.0 --repo owner/repo
gh verify release v0.9.0..v1.0.0 --repo owner/repo

# SLSA level selection (default: source-l1-build-l1)
gh verify pr 123 --repo owner/repo --slsa-level source-l3-build-l2

# Custom OPA policy for gate decisions
gh verify pr 123 --repo owner/repo --policy policy.rego

# Output formats: human (default), json, sarif
gh verify pr 123 --repo owner/repo --format sarif
```

Exit codes: `0` = pass, `1` = fail.

### GitHub Action

```yaml
- uses: HikaruEgashira/gh-verify@v0.5
  with:
    command: pr
    argument: ${{ github.event.pull_request.number }}
```

See [action.yml](action.yml) for full input/output details.

## Controls

Level selection via `--slsa-level` determines which SLSA controls are enforced.
Compliance controls always run alongside SLSA controls.

### SLSA v1.2

| Track | Level | Control |
|-------|-------|---------|
| Source | L1 | `review-independence`, `source-authenticity` |
| Source | L2 | `branch-history-integrity` |
| Source | L3 | `branch-protection-enforcement` |
| Source | L4 | `two-party-review` |
| Build | L1 | `build-provenance`, `required-status-checks` |
| Build | L2 | `hosted-build-platform`, `provenance-authenticity` |
| Build | L3 | `build-isolation` |

### SOC2 CC7/CC8

| Criteria | Control |
|----------|---------|
| CC7.1 (Traceability) | `issue-linkage`, `release-traceability` |
| CC7.2 (Anomaly detection) | `stale-review`, `security-file-change` |
| CC8.1 (Change management) | `pr-size`, `test-coverage`, `scoped-change`, `description-quality`, `merge-commit-policy`, `conventional-title` |

### NIST SSDF (SP 800-218) / OpenSSF Scorecard

| NIST Practice | Scorecard Check | Control |
|---------------|-----------------|---------|
| PW.7 (Vuln testing) | SAST | `sast-tool-presence` |
| PS.1 (Protect code) | Binary-Artifacts | `binary-artifact-check` |
| PS.1 (Protect code) | Pinned-Dependencies | `dependency-pinning` |
| PS.1 (Protect code) | Token-Permissions | `workflow-permissions` |

### Policy presets

```bash
gh verify pr 123 --repo owner/repo --policy nist-ssdf.rego   # NIST SSDF aligned
gh verify pr 123 --repo owner/repo --policy openssf.rego     # OpenSSF Scorecard aligned
gh verify pr 123 --repo owner/repo --policy oss.rego          # OSS/solo dev tolerant
```

## Architecture

Three-crate workspace:

- `gh-verify-core` — Pure verification logic. No I/O, no unsafe. Formally verified predicates.
- `gh-verify` — CLI binary with GitHub API integration and output formatting.
- `gh-verify-verif` — Creusot verification targets with `#[ensures]` specs.

## License

[MIT](LICENSE)
