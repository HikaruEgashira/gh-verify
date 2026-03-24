# Custom Policies

Every finding passes through an [OPA Rego](https://www.openpolicyagent.org/docs/latest/policy-language/) rule
(`data.verify.profile.map`) that decides the gate outcome.
The rule receives this input per finding:

| Field | Type | Example |
|-------|------|---------|
| `input.control_id` | string (kebab-case) | `"review-independence"` |
| `input.status` | `"satisfied"` \| `"violated"` \| `"indeterminate"` \| `"not_applicable"` | `"violated"` |
| `input.rationale` | string | `"PR author is the sole approver"` |
| `input.subjects` | list of strings | `["https://github.com/org/repo/pull/42"]` |

The rule must return `{"severity": "<info|warning|error>", "decision": "<pass|review|fail>"}`.

## Quick start

```rego
# .ghverify.rego — minimal custom policy
package verify.profile
import rego.v1

default map := {"severity": "error", "decision": "fail"}

map := {"severity": "info", "decision": "pass"} if { input.status == "satisfied" }
map := {"severity": "info", "decision": "pass"} if { input.status == "not_applicable" }
map := {"severity": "error", "decision": "fail"} if { input.status == "violated" }
map := {"severity": "warning", "decision": "review"} if { input.status == "indeterminate" }
```

```bash
gh verify pr 123 --repo org/repo --policy .ghverify.rego
```

## Common recipes

**Opt out `conventional-title`** (project does not use Conventional Commits):

```rego
map := {"severity": "info", "decision": "pass"} if {
    input.control_id == "conventional-title"
}
```

**Make `test-coverage` advisory** (review instead of fail):

```rego
map := {"severity": "warning", "decision": "review"} if {
    input.control_id == "test-coverage"
    input.status == "violated"
}
```

**Treat all indeterminate as review** (aiops-style):

```rego
map := {"severity": "warning", "decision": "review"} if {
    input.status == "indeterminate"
}
```

**Per-control overrides with a set** (SOC1-style advisory group):

```rego
advisory_controls := {"change-request-size", "scoped-change", "conventional-title", "test-coverage"}

map := {"severity": "warning", "decision": "review"} if {
    input.control_id in advisory_controls
    input.status != "satisfied"
    input.status != "not_applicable"
}
```

## Available control IDs

All 24 control IDs that can appear in `input.control_id`:

| Category | Control IDs |
|----------|-------------|
| Source track | `source-authenticity`, `review-independence`, `branch-history-integrity`, `branch-protection-enforcement`, `two-party-review` |
| Build track | `build-provenance`, `required-status-checks`, `hosted-build-platform`, `provenance-authenticity`, `build-isolation` |
| Dependencies track | `dependency-signature`, `dependency-provenance`, `dependency-signer-verified`, `dependency-completeness` |
| Compliance | `change-request-size`, `test-coverage`, `scoped-change`, `issue-linkage`, `stale-review`, `description-quality`, `merge-commit-policy`, `conventional-title`, `security-file-change`, `release-traceability` |
