<h1 align="center">gh-verify</h1>

<p align="center">
  GitHub SDLC health checker.
</p>

<p align="center">
  <a href="https://github.com/HikaruEgashira/libverify">libverify</a> · <a href="HACKING.md">Hacking</a>
</p>

---

gh-verify checks your pull requests, releases, and repository settings for common security and quality problems -- like missing code reviews, unsigned commits, or oversized PRs. It can enforce industry frameworks (SLSA, SOC2) or just basic hygiene.

Runs as a `gh` CLI extension, powered by [libverify](https://github.com/HikaruEgashira/libverify).

> [!NOTE]
>
> gh-verify follows semver. The 0.x series may introduce breaking changes between minor versions.
> Pin to a specific version in CI (e.g., `@v0.12`). The CLI interface, exit codes, and SARIF schema are stable.

## Why gh-verify?

GitHub branch protection rules enforce reviews and status checks. gh-verify goes further:

- **Detects stale reviews** -- code pushed after the last approval is flagged, not just whether a review exists
- **Checks commit signing** -- verifies all commits are cryptographically signed, not just that the branch is protected
- **Analyzes PR quality** -- size, scope, description, test coverage, conventional commit titles
- **Verifies dependencies** -- signatures, provenance, and completeness of the supply chain
- **Maps to compliance frameworks** -- SLSA L1-L4 and SOC2 CC7/CC8 controls, ready for audits

## Adoption Path

1. **Evaluate** -- Run `gh verify pr 42 --audit` on a few repos to see findings without blocking CI
2. **Tune** -- Use `--policy oss` or `--exclude` to suppress controls that do not apply to your project
3. **Enforce** -- Remove `--audit` and let gh-verify gate your CI pipeline
4. **Scale** -- Add the GitHub Action to your org-wide reusable workflow

## Quick Start

```bash
# 1. Install
gh extension install HikaruEgashira/gh-verify

# 2. Try it on any public repo
gh verify pr 6933 --repo expressjs/express

# 3. Try audit mode (reports without failing)
gh verify pr 42 --audit
```

## Usage

### CLI

```bash
# Verify a pull request (auto-detects repo from git remote)
gh verify pr 42

# Verify a release tag
gh verify release 0.15.7 --repo astral-sh/ruff

# Check repository security settings
gh verify repo --repo cli/cli --policy soc2

# Batch verify a PR range
gh verify pr '#100..#200'

# Audit mode: report without failing (useful for onboarding)
gh verify pr 42 --audit

# Quiet mode: suppress progress messages (useful for CI)
gh verify pr 42 --quiet

# Policy presets
gh verify pr 42 --policy oss        # OSS-friendly
gh verify pr 42 --policy slsa-l3    # SLSA Level 3

# Custom OPA policy file
gh verify pr 42 --policy policy.rego

# Exclude specific controls (see 'gh verify controls' for IDs)
gh verify pr 42 --exclude secret-scanning,conventional-title

# Output formats: human (default), json, sarif
gh verify pr 42 --format json
gh verify pr 42 --format sarif      # For GitHub Code Scanning
```

Exit codes: `0` = all controls pass, `1` = verification failure, `2` = infrastructure error (API/auth/network).

> **PowerShell users:** Use double quotes for range arguments: `gh verify pr "#100..#200"`

### GitHub Action

```yaml
- uses: HikaruEgashira/gh-verify@v0.12
  with:
    command: pr
    argument: ${{ github.event.pull_request.number }}

# With policy and exclusions
- uses: HikaruEgashira/gh-verify@v0.12
  with:
    command: repo
    policy: soc2
    exclude: secret-scanning,vulnerability-scanning
```

See [action.yml](action.yml) for full input/output details.

### Which policy should I use?

| Scenario | Recommended |
|----------|-------------|
| Solo / personal project | `--policy oss --exclude review-independence,two-party-review,stale-review,security-file-change,codeowners-coverage` |
| Open source project | `--policy oss` |
| Getting started / evaluation | `--audit` (no policy needed) |
| SOC2 audit preparation | `--policy soc2` |
| SLSA compliance | `--policy slsa-l1` through `slsa-l4` |
| Monorepo (multi-package) | `--exclude change-request-size,scoped-change` or `--policy oss --exclude change-request-size,scoped-change` |
| No specific requirements | omit `--policy` (uses `default`) |

> **Solo developers:** Some controls (review-independence, two-party-review, stale-review, security-file-change, codeowners-coverage) require a team and will always fail for solo projects. Exclude them to focus on actionable checks like PR size, test coverage, and dependency security.

> **Monorepos:** Controls like `change-request-size` and `scoped-change` may produce false positives for PRs that span multiple packages (e.g., API + frontend + shared library). These controls evaluate the entire PR without awareness of package boundaries. Use `--exclude change-request-size,scoped-change` to suppress them, or use `--only` to run only the controls relevant to your workflow.

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
| `default` | All controls strict -- uncertain or non-compliant results map to fail |
| `oss` | Allows unsigned commits and self-reviewed merges (maps to review instead of fail) |
| `aiops` | Maps all uncertain results to human review instead of fail |
| `soc1` | Strict on ICFR-relevant controls; informational on compliance controls |
| `soc2` | Strict on all CC6/CC7/CC8 controls; review on uncertain build-track results |
| `slsa-l1`..`slsa-l4` | Enforce SLSA source/build/dependencies controls at the specified level |

## Development

See [HACKING.md](HACKING.md) for build commands and contribution guide.

## License

[MIT](LICENSE)
