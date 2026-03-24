# Getting Started: gh-verify for Compliance & Security Teams

Your auditor asks: "Show me evidence that PR #4521 had independent review, signed commits, and passing CI before merge." You don't want to screenshot GitHub. Here's how to produce that evidence in one command.

## The Audit Scenario

It's quarter-end. You need to demonstrate SOC2 CC8.1 (change management) compliance for all changes shipped between v2.0 and v3.0.

```bash
gh extension install HikaruEgashira/gh-verify

gh verify pr 'v2.0.0..v3.0.0' \
  --repo your-org/your-repo \
  --policy soc2 \
  --format json \
  --with-evidence > audit-evidence-v3.json
```

One file. Every merged PR in that range, verified against SOC2 controls, with the raw evidence attached.

## What's in the Output

Each PR produces three layers:

**Findings** — what each control determined:

```
[review-independence]  pass [compliant]: At least one approver is independent
[issue-linkage]        fail [exception]: no issue or ticket references found
[stale-review]         pass [compliant]: all approvals postdate the latest source revision
```

**Outcomes** — the policy's verdict, using audit-native language:

| Label | Meaning |
|-------|---------|
| `compliant` | Control satisfied |
| `observation` | Review recommended |
| `exception` | Remediation required |

**Evidence** — the raw data behind each verdict. Add `--with-evidence` and the JSON includes exactly what the controls evaluated:

```json
{
  "approval_decisions": {
    "state": "complete",
    "value": [
      { "actor": "jonchurch", "disposition": "approved",
        "submitted_at": "2025-12-01T18:45:47Z" }
    ]
  },
  "source_revisions": {
    "state": "complete",
    "value": [
      { "id": "ebd3876...", "authored_by": "UlisesGascon",
        "authenticity": { "verified": true, "mechanism": "valid" } }
    ]
  },
  "check_runs": {
    "state": "complete",
    "value": [
      { "name": "Lint", "conclusion": "success" },
      { "name": "Node.js 16 - ubuntu-latest", "conclusion": "success" }
    ]
  }
}
```

Every evidence field carries a `state`: `complete`, `partial` (with gaps explaining what's missing), `missing`, or `not_applicable`. When a control returns `indeterminate`, the gaps tell you exactly why.

## SOC2 Control Mapping

The `soc2` policy maps findings to Trust Services Criteria:

| Criteria | What gh-verify checks |
|----------|----------------------|
| **CC7.1** Traceability | Issue linkage, release traceability |
| **CC7.2** Anomaly detection | Stale reviews, security file changes |
| **CC8.1** Change management | Change size, test coverage, scoped changes, description quality |

## SLSA Levels

The `slsa-l1` through `slsa-l4` presets enforce progressively stricter requirements across all three SLSA tracks: Source, Build, and Dependencies.

```bash
gh verify pr 42 --repo your-org/your-repo --policy slsa-l3
```

## Continuous Monitoring

For ongoing compliance, add the GitHub Action with SARIF upload. Findings appear as Code Scanning alerts — developers see them inline during review, auditors see them in the Security tab.

```yaml
- uses: HikaruEgashira/gh-verify@v0.9
  with:
    command: pr
    argument: ${{ github.event.pull_request.number }}
    policy: soc2
    upload-sarif: true
```

Point-in-time batch audits when needed. Continuous checks on every PR in between.
