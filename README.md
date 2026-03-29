<h1 align="center">gh-verify</h1>

<p align="center">
  GitHub SDLC health checker.
</p>

<p align="center">
  <a href="https://github.com/HikaruEgashira/libverify">libverify</a> · <a href="HACKING.md">Hacking</a>
</p>

---

gh-verify verifies that pull requests and releases follow
supply chain security and compliance practices (SLSA, SOC2, and custom OPA policies).
It runs as a `gh` CLI extension, powered by [libverify](https://github.com/HikaruEgashira/libverify).

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

# Verify repository security posture
gh verify repo --repo cli/cli --policy soc2

# Verify a release range
gh verify release v5.2.0..v5.2.1 --repo expressjs/express

# Policy preset (includes SLSA levels)
gh verify pr 6933 --repo expressjs/express --policy slsa-l3
gh verify release 0.15.7 --repo astral-sh/ruff --policy soc2

# Custom OPA policy file
gh verify pr 6933 --repo expressjs/express --policy policy.rego

# Exclude specific controls
gh verify pr 42 --exclude secret-scanning,conventional-title

# Output formats: human (default), json, sarif
gh verify release 1.94.0 --repo rust-lang/rust --format json
```

Exit codes: `0` = pass, `1` = fail.

### GitHub Action

```yaml
- uses: HikaruEgashira/gh-verify@v0.11
  with:
    command: pr
    argument: ${{ github.event.pull_request.number }}

# With policy and exclusions
- uses: HikaruEgashira/gh-verify@v0.11
  with:
    command: repo
    policy: soc2
    exclude: secret-scanning,vulnerability-scanning
```

See [action.yml](action.yml) for full input/output details.

## Controls & Policies

Policy selection via `--policy` determines which controls are enforced and how strictly.
Full details are in [libverify](https://github.com/HikaruEgashira/libverify).

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
| Dependencies | L1 | `dependency-signature` |
| Dependencies | L2 | `dependency-provenance` |
| Dependencies | L3 | `dependency-signer-verified` |
| Dependencies | L4 | `dependency-completeness` |

### SOC2 CC7/CC8

| Criteria | Control |
|----------|---------|
| CC7.1 (Traceability) | `issue-linkage`, `release-traceability` |
| CC7.2 (Anomaly detection) | `stale-review`, `security-file-change` |
| CC8.1 (Change management) | `change-request-size`, `test-coverage`, `scoped-change`, `description-quality`, `merge-commit-policy`, `conventional-title` |

### Policy presets

| Preset | Description |
|--------|-------------|
| `default` | All controls strict (indeterminate/violated → fail) |
| `oss` | Tolerates unsigned commits and self-reviewed merges |
| `aiops` | Escalates all indeterminate to human review instead of fail |
| `soc1` | Strict on ICFR-relevant controls; advisory on compliance controls |
| `soc2` | Strict on all CC6/CC7/CC8 controls; review on build-track indeterminate |
| `slsa-l1`..`slsa-l4` | Enforce SLSA source/build/dependencies controls at the specified level |

## Development

See [HACKING.md](HACKING.md) for build commands and contribution guide.

## License

[MIT](LICENSE)
