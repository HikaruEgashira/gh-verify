# Fail-Open Design Guide

How to run gh-verify in CI without blocking deployments on infrastructure failures.

## Exit Code Contract

| Code | Meaning | Typical cause |
|------|---------|---------------|
| `0` | All controls pass | Healthy PR/release |
| `1` | Verification failure | One or more controls returned `fail` |
| `2` | Infrastructure error | GitHub API outage, auth failure, network timeout |

The `--audit` flag forces exit `0` regardless of verification results, turning gh-verify into a reporter that never blocks CI.

## Pattern 1: Fail-Open (skip on infra errors only)

Gate on verification failures (exit 1) but tolerate API outages (exit 2).

### Shell wrapper

```bash
#!/usr/bin/env bash
set +e
gh verify pr "$PR_NUMBER" --format json --repo "$REPO"
rc=$?
set -e

case $rc in
  0) echo "All controls passed" ;;
  1) echo "::error::Verification failed"; exit 1 ;;
  2) echo "::warning::Infrastructure error — skipping verification"; exit 0 ;;
  *) echo "::warning::Unexpected exit code $rc — skipping"; exit 0 ;;
esac
```

### GitHub Actions

```yaml
- name: Verify PR (fail-open)
  id: verify
  shell: bash
  run: |
    set +e
    gh verify pr ${{ github.event.pull_request.number }} \
      --format json --repo ${{ github.repository }}
    rc=$?
    set -e
    echo "exit-code=$rc" >> "$GITHUB_OUTPUT"
    if [ "$rc" -eq 1 ]; then
      echo "::error::gh-verify found failing controls"
      exit 1
    fi
    if [ "$rc" -eq 2 ]; then
      echo "::warning::gh-verify infrastructure error — skipping"
    fi
```

## Pattern 2: Audit Mode (report only)

No CI blocking at all. Useful for initial rollout or incident response.

```yaml
- uses: HikaruEgashira/gh-verify@v0
  with:
    command: pr
    argument: ${{ github.event.pull_request.number }}
    audit: "true"
    format: sarif

- uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: results.sarif
```

Findings appear as Code Scanning alerts in the Security tab. Developers see inline annotations during review; nothing blocks merge.

## Pattern 3: Fail-Close (strict enforcement)

Every non-zero exit blocks the pipeline. Use for production-critical repos.

```yaml
- uses: HikaruEgashira/gh-verify@v0
  with:
    command: pr
    argument: ${{ github.event.pull_request.number }}
    policy: soc2
```

No `audit`, no `continue-on-error`. Exit 1 and exit 2 both fail the job.

## Recommended Setup by Environment

| Environment | Pattern | Rationale |
|-------------|---------|-----------|
| Development | Audit mode (`--audit`) | Zero friction; findings visible as alerts |
| Staging | Fail-open (exit 2 skipped) | Catches real violations; API outages do not block |
| Production | Fail-close (strict) | All errors block merge; requires healthy infra |

### Full workflow example

```yaml
name: SDLC Verification
on:
  pull_request:

jobs:
  verify:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      security-events: write
    env:
      GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
    steps:
      - uses: actions/checkout@v4

      # Determine enforcement level from target branch
      - name: Set enforcement
        id: env
        run: |
          case "${{ github.base_ref }}" in
            main|master)  echo "mode=strict"    >> "$GITHUB_OUTPUT" ;;
            staging)      echo "mode=fail-open" >> "$GITHUB_OUTPUT" ;;
            *)            echo "mode=audit"     >> "$GITHUB_OUTPUT" ;;
          esac

      # Audit mode for dev branches
      - name: Verify (audit)
        if: steps.env.outputs.mode == 'audit'
        uses: HikaruEgashira/gh-verify@v0
        with:
          command: pr
          argument: ${{ github.event.pull_request.number }}
          audit: "true"
          format: sarif

      # Fail-open for staging
      - name: Verify (fail-open)
        if: steps.env.outputs.mode == 'fail-open'
        shell: bash
        run: |
          set +e
          gh verify pr ${{ github.event.pull_request.number }} \
            --format json --repo ${{ github.repository }}
          rc=$?
          set -e
          if [ "$rc" -eq 1 ]; then
            echo "::error::Verification failed"
            exit 1
          fi
          [ "$rc" -eq 2 ] && echo "::warning::Infra error — skipped"

      # Fail-close for production
      - name: Verify (strict)
        if: steps.env.outputs.mode == 'strict'
        uses: HikaruEgashira/gh-verify@v0
        with:
          command: pr
          argument: ${{ github.event.pull_request.number }}
          policy: soc2
```

## Monitoring

Regardless of pattern, capture exit code 2 occurrences to detect persistent API issues:

```yaml
- name: Track infra errors
  if: always()
  run: |
    if [ "${{ steps.verify.outputs.exit-code }}" = "2" ]; then
      echo "::warning::gh-verify infrastructure error detected"
      # Forward to your observability stack (Datadog, PagerDuty, etc.)
    fi
```
