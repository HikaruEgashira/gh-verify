<h1 align="center">gh-verify</h1>

<p align="center">
  GitHub SDLC health checker.
</p>

<p align="center">
  <a href="HACKING.md">Hacking</a> · <a href="benchmarks/README.md">Benchmarks</a>
</p>

---

gh-verify verifies that pull requests and releases follow
supply chain security and compliance practices (SLSA, SOC2, and custom OPA policies).
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
# Install
gh extension install HikaruEgashira/gh-verify

# Verify a pull request
gh verify pr 6933 --repo expressjs/express

# Verify a release tag
gh verify release 0.15.7 --repo astral-sh/ruff

# Verify a release range
gh verify release v5.2.0..v5.2.1 --repo expressjs/express

# SLSA level selection (default: source-l1-build-l1)
gh verify pr 6933 --repo expressjs/express --slsa-level source-l3-build-l2

# Policy preset
gh verify release 0.15.7 --repo astral-sh/ruff --policy soc2

# Custom OPA policy file
gh verify pr 6933 --repo expressjs/express --policy policy.rego

# Output formats: human (default), json, sarif
gh verify release 1.94.0 --repo rust-lang/rust --format json
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

### Policy presets

| Preset | Description |
|--------|-------------|
| `default` | All controls strict (indeterminate/violated → fail) |
| `oss` | Tolerates unsigned commits and self-reviewed merges |
| `aiops` | Escalates all indeterminate to human review instead of fail |
| `soc1` | Strict on ICFR-relevant controls; advisory on dev-quality controls |
| `soc2` | Strict on all CC6/CC7/CC8 controls; review on build-track indeterminate |

```bash
gh verify pr 6933 --repo expressjs/express --policy oss
gh verify release 0.15.7 --repo astral-sh/ruff --policy soc2
```

## Development

See [HACKING.md](HACKING.md) for architecture, build commands, and contribution guide.

## License

[MIT](LICENSE)
