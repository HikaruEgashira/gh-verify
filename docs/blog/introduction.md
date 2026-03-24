# gh-verify — Lint Your Software Supply Chain

```bash
gh extension install HikaruEgashira/gh-verify
gh verify pr 6933 --repo expressjs/express
```

That's it. One command tells you whether a pull request meets SLSA v1.2 and SOC2 standards. No SaaS contract. No onboarding. Just a `gh` extension.

## Real-World Example: Express.js

Let's verify [express#6933](https://github.com/expressjs/express/pull/6933), a recent PR on one of the most widely-used Node.js frameworks.

```
$ gh verify pr 6933 --repo expressjs/express

[source-authenticity]           pass [compliant]: All revisions carry authenticity evidence
[review-independence]           pass [compliant]: At least one approver is independent from both author and requester
[branch-history-integrity]      pass [compliant]: All 2 commit(s) form a linear history
[branch-protection-enforcement] pass [compliant]: CI checks passed and independent review approved
[two-party-review]              fail [exception]: Only 1 independent approver(s) found; at least 2 required
[required-status-checks]        pass [compliant]: 28 check run(s) passed
[hosted-build-platform]         pass [compliant]: All 28 build platform(s) are hosted
[build-isolation]               fail [exception]: Build isolation violation(s): github-advanced-security (signing key not isolated)
[change-request-size]           pass [compliant]: Change request size is acceptable (100 lines across 4 files)
[test-coverage]                 fail [exception]: 1 source file(s) changed without matching test updates: lib/utils.js
[scoped-change]                 pass [compliant]: change request is well-scoped
[issue-linkage]                 fail [exception]: no issue or ticket references found
[description-quality]           pass [compliant]: description present (1575 chars)
[conventional-title]            fail [exception]: title does not follow Conventional Commits format

Summary: 12 pass, 0 review, 5 fail
```

5 fails. But wait — this is Express.js, an established OSS project. Should we really enforce two-party review and conventional commits on a mature open-source repo?

### Same PR, Different Policy

```
$ gh verify pr 6933 --repo expressjs/express --policy oss

[two-party-review]    review [observation]: Only 1 independent approver(s)
[issue-linkage]       review [observation]: no issue or ticket references found
[conventional-title]  review [observation]: title does not follow Conventional Commits format

Summary: 12 pass, 3 review, 2 fail
```

The `oss` policy downgrades three findings from **fail** to **review**. Two-party review, issue linkage, and conventional titles become observations rather than blockers — reasonable for open-source workflows where a single trusted maintainer often suffices.

The two remaining fails (`build-isolation`, `test-coverage`) stay as fails because they're genuine supply chain risks regardless of context.

## The Problem

GitHub gives you branch protection, required reviews, and status checks. But it can't answer: **"Is this PR actually safe to merge?"**

- Is the reviewer independent from the author?
- Are all commits signed?
- Is the change scope reasonable?
- Did every PR in this release pass verification?

Too tedious to check manually. Too complex to script yourself.

## What It Checks

28 built-in controls, modeled after how ESLint checks code quality — but for your development process.

**SLSA v1.2** — Source integrity (L1–L4), build provenance (L1–L3), dependency verification (L1–L4).

**SOC2 CC7/CC8** — Issue linkage, stale review detection, change size, test coverage, scoped changes, conventional commits.

**Repository posture** — CODEOWNERS coverage, secret scanning, vulnerability scanning, security policy.

## Policies: Same Facts, Different Verdicts

gh-verify ships an OPA Rego policy engine with presets — `oss`, `soc2`, `slsa-l1` through `slsa-l4`, `aiops` — or bring your own `.rego` file:

```bash
gh verify pr 42 --policy soc2
gh verify pr 42 --policy my-org-policy.rego
```

## Show Your Evidence

Add `--with-evidence` and the output includes the raw data behind every verdict — who approved, which commits were signed, what CI checks ran. Verdicts plus proof, in one JSON artifact.

```bash
gh verify pr 6933 --repo expressjs/express --format json --with-evidence
```

## Formally Verified Rule Logic

The decision predicates — "is this reviewer independent?", "is this commit signed?" — are **mathematically proven correct** via [Creusot](https://github.com/creusot-rs/creusot) and SMT solvers. 15+ predicates verified.

Other tools use cryptographic signatures to prevent data tampering. gh-verify proves **the rules themselves** don't lie.

## CI/CD Native

```yaml
- uses: HikaruEgashira/gh-verify@v0.9
  with:
    command: pr
    argument: ${{ github.event.pull_request.number }}
    policy: soc2
    upload-sarif: true
```

SARIF output feeds directly into GitHub Code Scanning. No dashboard needed.

Batch verification for audits — PR ranges, tag ranges, date ranges:

```bash
gh verify pr 'v1.0.0..v2.0.0' --repo org/repo --policy soc2 --format json
```

## Architecture

gh-verify is a thin CLI shell (~300 LOC). All verification logic lives in [libverify](https://github.com/HikaruEgashira/libverify), a platform-agnostic engine. Swap the GitHub adapter for GitLab or Bitbucket — the controls, policies, and output formatters stay the same.

---

```bash
gh extension install HikaruEgashira/gh-verify
```

[GitHub](https://github.com/HikaruEgashira/gh-verify) · MIT License
