# Getting Started: gh-verify for Platform Engineers

You own the CI/CD pipeline. Here's a copy-paste workflow and the knobs you'll want to turn.

## The Workflow

```yaml
# .github/workflows/verify.yml
name: SDLC Verify
on:
  pull_request:
    types: [opened, synchronize, reopened]

jobs:
  verify:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      pull-requests: read
      checks: read
      security-events: write
    steps:
      - uses: HikaruEgashira/gh-verify@v0.9
        id: verify
        with:
          command: pr
          argument: ${{ github.event.pull_request.number }}
          policy: default
          upload-sarif: true
```

28 controls run on every PR. Results show up as Code Scanning alerts in the PR diff. Done.

## Wiring Outputs

The action exposes four outputs: `result` (`pass` | `review` | `fail`), `pass-count`, `review-count`, `fail-count`.

Gate a deployment:

```yaml
      - uses: HikaruEgashira/gh-verify@v0.9
        id: verify
        with:
          command: pr
          argument: ${{ github.event.pull_request.number }}

      - if: steps.verify.outputs.result == 'fail'
        run: |
          echo "::error::SDLC verification failed (${{ steps.verify.outputs.fail-count }} controls)"
          exit 1
```

Post to Slack, feed Datadog, block a merge queue — it's just step outputs.

## Release Gate

Add a second workflow for releases. This traces all commits between the previous tag and the new one, resolves the associated PRs, and verifies each:

```yaml
# .github/workflows/release-verify.yml
name: Release Verify
on:
  release:
    types: [created]

jobs:
  verify:
    runs-on: ubuntu-latest
    steps:
      - uses: HikaruEgashira/gh-verify@v0.9
        with:
          command: release
          argument: ${{ github.event.release.tag_name }}
          policy: soc2
          upload-sarif: true
```

## Tuning the Policy

`default` is strict. Start there to see what breaks, then adjust.

| Preset | Behavior |
|--------|----------|
| `default` | Everything strict. Start here. |
| `oss` | Tolerates self-merge, unsigned commits. |
| `soc2` | Strict on CC7/CC8, advisory on compliance. |
| `slsa-l2` | Enforces SLSA Level 2 source + build controls. |

When a preset is close but not right, override specific controls with a `.rego` file committed to the repo:

```rego
# .ghverify.rego
package verify.profile
import rego.v1

default map := {"severity": "error", "decision": "fail"}

map := {"severity": "info", "decision": "pass"} if { input.status == "satisfied" }
map := {"severity": "info", "decision": "pass"} if { input.status == "not_applicable" }
map := {"severity": "warning", "decision": "review"} if { input.status == "indeterminate" }

# Your team doesn't use Conventional Commits
map := {"severity": "info", "decision": "pass"} if {
    input.control_id == "conventional-title"
}
```

```yaml
          policy: .ghverify.rego
```

Policy lives with the code. PR to change it, review to approve it.
