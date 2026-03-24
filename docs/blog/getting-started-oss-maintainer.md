# Getting Started: gh-verify for OSS Maintainers

Let's run gh-verify against Express.js and see what it finds.

## Express.js PR #6933, Unfiltered

```bash
gh extension install HikaruEgashira/gh-verify
gh verify pr 6933 --repo expressjs/express
```

```
[source-authenticity]           pass: All revisions carry authenticity evidence
[review-independence]           pass: At least one approver is independent
[branch-history-integrity]      pass: All 2 commit(s) form a linear history
[branch-protection-enforcement] pass: CI checks passed and independent review approved
[two-party-review]              fail: Only 1 independent approver(s); at least 2 required
[required-status-checks]        pass: 28 check run(s) passed
[hosted-build-platform]         pass: All 28 build platform(s) are hosted
[build-isolation]               fail: github-advanced-security (signing key not isolated)
[change-request-size]           pass: 100 lines across 4 files
[test-coverage]                 fail: lib/utils.js changed without matching test updates
[scoped-change]                 pass: change request is well-scoped
[issue-linkage]                 fail: no issue or ticket references found
[conventional-title]            fail: title does not follow Conventional Commits format

Summary: 12 pass, 0 review, 5 fail
```

5 fails. But Express doesn't require two-party review or Conventional Commits. These aren't defects — they're policy mismatches.

## Same PR, `oss` Policy

```bash
gh verify pr 6933 --repo expressjs/express --policy oss
```

Three findings flip from `fail` to `review`:

```
[two-party-review]    fail → review
[issue-linkage]       fail → review
[conventional-title]  fail → review
```

The two that stay `fail` — `build-isolation` and `test-coverage` — are real gaps regardless of project style. A source file changed without test updates. A third-party check runs without isolated signing keys.

**The `oss` policy doesn't lower the bar. It moves it to where it matters for open-source.**

## Adding It to CI Without Blocking Anyone

```yaml
# .github/workflows/verify.yml
name: SDLC Verify
on:
  pull_request:
    types: [opened, synchronize]

jobs:
  verify:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      pull-requests: read
      checks: read
    steps:
      - uses: HikaruEgashira/gh-verify@v0.9
        with:
          command: pr
          argument: ${{ github.event.pull_request.number }}
          policy: oss
    continue-on-error: true
```

`continue-on-error: true` — contributors see results, nothing is blocked. Start with visibility. Enforce later, if ever.

## Making It Yours

Skip controls your project doesn't care about:

```rego
# .ghverify.rego
package verify.profile
import rego.v1

default map := {"severity": "error", "decision": "fail"}

map := {"severity": "info", "decision": "pass"} if { input.status == "satisfied" }
map := {"severity": "info", "decision": "pass"} if { input.status == "not_applicable" }
map := {"severity": "warning", "decision": "review"} if { input.status == "indeterminate" }

skip := {"conventional-title", "issue-linkage"}
map := {"severity": "info", "decision": "pass"} if { input.control_id in skip }
```

## Pre-Release Sanity Check

```bash
gh verify release v2.0.0 --policy oss
```

One command before you tag. Shows whether the changes since your last release hold up.
