# Compliance Framework Mapping

gh-verify's 38 SDLC controls map to multiple compliance frameworks.
This document provides cross-reference tables for audit preparation.

## How to Read This Document

- **Control ID**: The identifier used in `gh verify controls` and `--exclude` flags
- **Status**: How the control maps to each framework requirement
  - **Direct**: The control directly satisfies the requirement
  - **Partial**: The control contributes to but does not fully satisfy the requirement
  - **Indirect**: The control provides supporting evidence

## SLSA v1.2 (Existing)

Already built-in via `--policy slsa-l1` through `--policy slsa-l4`.
See [README.md](../README.md#slsa-v12) for the full mapping.

## SOC 2 Type II (Existing)

Already built-in via `--policy soc2`.
See [README.md](../README.md#soc2-cc7cc8) for the full mapping.

---

## NIST SP 800-53 Rev. 5

Relevant control families: CM (Configuration Management), SA (System and Services Acquisition), SI (System and Information Integrity), AU (Audit and Accountability).

| NIST Control | Title | gh-verify Control(s) | Mapping |
|---|---|---|---|
| CM-2 | Baseline Configuration | `branch-protection-enforcement` | Direct |
| CM-3 | Configuration Change Control | `change-request-size`, `scoped-change`, `description-quality`, `review-independence` | Direct |
| CM-3(2) | Testing, Validation, Analysis | `test-coverage`, `required-status-checks` | Direct |
| CM-4 | Impact Analysis | `scoped-change`, `security-file-change` | Partial |
| CM-5 | Access Restrictions for Change | `branch-protection-enforcement`, `codeowners-coverage` | Direct |
| CM-14 | Signed Components | `source-authenticity`, `dependency-signature` | Direct |
| SA-10 | Developer Configuration Mgmt | `branch-history-integrity`, `merge-commit-policy` | Direct |
| SA-11 | Developer Testing | `test-coverage`, `required-status-checks` | Direct |
| SA-12 | Supply Chain Protection | `dependency-signature`, `dependency-provenance`, `dependency-signer-verified`, `dependency-completeness` | Direct |
| SA-15 | Development Process | `review-independence`, `two-party-review`, `conventional-title` | Direct |
| SI-7 | Software/Info Integrity | `source-authenticity`, `build-provenance`, `provenance-authenticity` | Direct |
| SI-7(1) | Integrity Checks | `dependency-signature`, `build-provenance` | Direct |
| SI-7(6) | Cryptographic Protection | `source-authenticity`, `provenance-authenticity` | Direct |
| SI-7(15) | Code Authentication | `source-authenticity` | Direct |
| SI-10 | Information Input Validation | `description-quality`, `conventional-title` | Indirect |
| AU-10 | Non-repudiation | `source-authenticity`, `review-independence` | Direct |
| AU-12 | Audit Record Generation | `issue-linkage`, `release-traceability` | Partial |
| SR-3 | Supply Chain Controls | `dependency-signature`, `dependency-provenance`, `dependency-completeness` | Direct |
| SR-4 | Provenance | `build-provenance`, `dependency-provenance`, `provenance-authenticity` | Direct |
| SR-11 | Component Authenticity | `dependency-signer-verified`, `source-authenticity` | Direct |
| CM-3(6) | Cryptography Management | `secret-scanning-push-protection` | Direct |
| CM-5(1) | Automated Access Enforcement | `branch-protection-admin-enforcement` | Direct |
| CM-3(4) | Security/Privacy Rep | `dismiss-stale-reviews-on-push` | Partial |
| SA-12(2) | Supplier Assessments | `actions-pinned-dependencies`, `dependency-license-compliance` | Direct |
| SA-12(14) | Identity and Traceability | `sbom-attestation`, `release-asset-attestation` | Direct |
| SI-7(12) | Integrity Verification | `code-scanning-alerts-resolved` | Direct |
| CM-7(5) | Authorized Software | `environment-protection-rules`, `privileged-workflow-detection` | Direct |

### FedRAMP Applicability

FedRAMP leverages NIST SP 800-53. The controls above apply to FedRAMP Moderate and High baselines.
For FedRAMP readiness, combine gh-verify with:
- `--policy soc2` for CC7/CC8 coverage
- `--policy slsa-l2` (Moderate) or `--policy slsa-l3` (High) for supply chain

---

## PCI DSS v4.0

Relevant requirements: 6 (Develop and Maintain Secure Systems and Software).

| PCI DSS Req | Title | gh-verify Control(s) | Mapping |
|---|---|---|---|
| 6.2.1 | Software developed securely | `review-independence`, `test-coverage`, `required-status-checks` | Direct |
| 6.2.2 | Training for secure development | — | Not covered (organizational) |
| 6.2.3 | Code review before release | `review-independence`, `two-party-review`, `stale-review` | Direct |
| 6.2.3.1 | Automated code review tools | `required-status-checks`, `vulnerability-scanning` | Partial |
| 6.2.4 | Software engineering techniques | `scoped-change`, `merge-commit-policy`, `conventional-title` | Indirect |
| 6.3.1 | Security vulnerabilities identified/managed | `vulnerability-scanning`, `secret-scanning` | Direct |
| 6.3.2 | Software inventory maintained | `dependency-signature`, `dependency-provenance` | Partial |
| 6.3.3 | Patches/updates applied timely | `vulnerability-scanning` | Partial |
| 6.4.1 | Public-facing web apps protected | — | Not covered (runtime) |
| 6.4.2 | Automated technical solution for web apps | — | Not covered (runtime) |
| 6.5.1 | Change control procedures | `change-request-size`, `description-quality`, `issue-linkage`, `review-independence` | Direct |
| 6.5.2 | Rollback upon failure | `release-traceability`, `branch-history-integrity` | Partial |
| 6.5.3 | Pre-production test environments | `hosted-build-platform`, `build-isolation` | Indirect |
| 6.5.4 | Separation of duties | `review-independence`, `two-party-review`, `codeowners-coverage` | Direct |
| 6.5.5 | Live PANs not used in test | — | Not covered (data-level) |
| 6.5.6 | Test data/accounts removed | — | Not covered (data-level) |
| 6.3.1.1 | Push protection for secrets | `secret-scanning-push-protection` | Direct |
| 6.5.4.1 | Admin enforcement on branches | `branch-protection-admin-enforcement` | Direct |
| 6.2.3.2 | Stale review dismissal | `dismiss-stale-reviews-on-push` | Partial |
| 6.3.2.1 | Pinned CI dependencies | `actions-pinned-dependencies` | Direct |
| 6.5.3.1 | Environment protection | `environment-protection-rules` | Direct |
| 6.3.1.2 | Code scanning resolution | `code-scanning-alerts-resolved` | Direct |
| 6.3.2.2 | Dependency license compliance | `dependency-license-compliance` | Direct |
| 6.3.2.3 | Software bill of materials | `sbom-attestation` | Partial |
| 6.3.2.4 | Release asset attestation | `release-asset-attestation` | Partial |
| 6.2.3.3 | Privileged workflow detection | `privileged-workflow-detection` | Direct |

### PCI DSS Quick Start

```bash
# PCI DSS focused verification
gh verify pr 42 --policy soc2 --exclude secret-scanning
# Add vulnerability scanning check
gh verify repo --repo myorg/payment-service --policy soc2
```

---

## ISMAP (Information system Security Management and Assessment Program)

ISMAP is based on JIS Q 27001 (ISO/IEC 27001) and JIS Q 27017 (ISO/IEC 27017).
The following maps gh-verify controls to ISMAP management criteria categories.

| ISMAP Category | Criteria | gh-verify Control(s) | Mapping |
|---|---|---|---|
| 12.1 | Operational procedures and responsibilities | `branch-protection-enforcement`, `codeowners-coverage` | Direct |
| 12.1.2 | Change management | `change-request-size`, `review-independence`, `description-quality`, `scoped-change` | Direct |
| 12.1.4 | Separation of environments | `hosted-build-platform`, `build-isolation` | Partial |
| 12.5 | Control of operational software | `source-authenticity`, `build-provenance` | Direct |
| 12.6 | Technical vulnerability management | `vulnerability-scanning`, `secret-scanning`, `dependency-signature` | Direct |
| 14.2.1 | Secure development policy | `review-independence`, `required-status-checks`, `test-coverage` | Direct |
| 14.2.2 | System change control procedures | `change-request-size`, `description-quality`, `issue-linkage`, `merge-commit-policy` | Direct |
| 14.2.3 | Technical review after platform changes | `stale-review`, `security-file-change` | Direct |
| 14.2.5 | Secure system engineering principles | `branch-protection-enforcement`, `source-authenticity` | Partial |
| 14.2.6 | Secure development environment | `build-isolation`, `hosted-build-platform` | Direct |
| 14.2.8 | System security testing | `test-coverage`, `required-status-checks` | Direct |
| 14.2.9 | System acceptance testing | `release-traceability` | Partial |
| 15.1.2 | Addressing security in supplier agreements | `dependency-provenance`, `dependency-signer-verified` | Direct |
| 15.2.1 | Monitoring and review of supplier services | `dependency-completeness`, `vulnerability-scanning` | Partial |
| 16.1.2 | Reporting information security events | `security-file-change`, `stale-review` | Indirect |
| 12.6.1 | Secret push protection | `secret-scanning-push-protection` | Direct |
| 14.2.5.1 | Admin branch enforcement | `branch-protection-admin-enforcement` | Direct |
| 14.2.3.1 | Dismiss stale reviews on push | `dismiss-stale-reviews-on-push` | Direct |
| 14.2.6.1 | Pinned CI/CD dependencies | `actions-pinned-dependencies` | Partial |
| 12.1.3 | Environment protection rules | `environment-protection-rules` | Direct |
| 12.6.2 | Code scanning alert resolution | `code-scanning-alerts-resolved` | Direct |
| 15.1.3 | Dependency license compliance | `dependency-license-compliance` | Partial |
| 14.2.9.1 | SBOM attestation | `sbom-attestation` | Partial |
| 14.2.9.2 | Release asset attestation | `release-asset-attestation` | Partial |
| 12.1.5 | Privileged workflow detection | `privileged-workflow-detection` | Direct |

### ISMAP Note

ISMAP requires the **cloud service provider** to be assessed, not just the development tools.
gh-verify provides evidence for development process controls within ISMAP scope.
For full ISMAP compliance, combine with organizational controls (access management, incident response, etc.).

---

## TISAX (Trusted Information Security Assessment Exchange)

TISAX is based on the VDA ISA (Information Security Assessment) catalog, aligned with ISO 27001.

| VDA ISA Control | Title | gh-verify Control(s) | Mapping |
|---|---|---|---|
| 1.3.1 | Change management | `change-request-size`, `review-independence`, `description-quality` | Direct |
| 1.3.2 | Separation of duties | `review-independence`, `two-party-review`, `codeowners-coverage` | Direct |
| 1.6.1 | Cryptographic controls | `source-authenticity`, `dependency-signature` | Direct |
| 2.1.1 | Asset management | `dependency-provenance`, `dependency-completeness` | Partial |
| 3.1.1 | Secure development lifecycle | `required-status-checks`, `test-coverage`, `review-independence` | Direct |
| 3.1.2 | Development environment security | `build-isolation`, `hosted-build-platform` | Direct |
| 3.1.3 | Source code management | `branch-protection-enforcement`, `branch-history-integrity`, `source-authenticity` | Direct |
| 4.1.1 | Supplier evaluation | `dependency-signer-verified`, `dependency-provenance` | Direct |
| 4.1.2 | Supply chain risk management | `dependency-signature`, `dependency-completeness`, `vulnerability-scanning` | Direct |
| 5.2.6 | Anomaly detection | `stale-review`, `security-file-change` | Partial |
| 1.6.2 | Secret push protection | `secret-scanning-push-protection` | Direct |
| 3.1.3.1 | Admin branch enforcement | `branch-protection-admin-enforcement` | Direct |
| 1.3.3 | Dismiss stale reviews on push | `dismiss-stale-reviews-on-push` | Direct |
| 4.1.2.1 | Pinned CI/CD dependencies | `actions-pinned-dependencies` | Direct |
| 3.1.2.1 | Environment protection rules | `environment-protection-rules` | Direct |
| 4.1.2.2 | Code scanning alert resolution | `code-scanning-alerts-resolved` | Direct |
| 4.1.3 | Dependency license compliance | `dependency-license-compliance` | Direct |
| 2.1.2 | SBOM attestation | `sbom-attestation` | Direct |
| 2.1.3 | Release asset attestation | `release-asset-attestation` | Direct |
| 5.2.7 | Privileged workflow detection | `privileged-workflow-detection` | Direct |

### TISAX Assessment Levels

| AL | Applicability | Recommended Policy |
|---|---|---|
| AL1 (Normal) | `--policy default` | Standard hygiene |
| AL2 (High) | `--policy soc2` | Strict change management |
| AL3 (Very High) | `--policy slsa-l3 --policy soc2` | Maximum verification |

---

## UNECE WP.29 / UN-R155 (Automotive Cybersecurity)

UN-R155 requires a Cybersecurity Management System (CSMS) covering the vehicle lifecycle.
gh-verify addresses the **software development phase** of CSMS.

| UN-R155 Clause | Requirement | gh-verify Control(s) | Mapping |
|---|---|---|---|
| 7.2.2.2(a) | Processes for identifying risks | `vulnerability-scanning`, `security-file-change` | Partial |
| 7.2.2.2(c) | Processes for verifying security is managed | `review-independence`, `required-status-checks`, `test-coverage` | Direct |
| 7.2.2.3 | Supply chain risk management | `dependency-signature`, `dependency-provenance`, `dependency-signer-verified`, `dependency-completeness` | Direct |
| 7.2.2.5(a) | Security design in development | `branch-protection-enforcement`, `build-isolation` | Partial |
| 7.2.2.5(b) | Security testing in development | `test-coverage`, `required-status-checks` | Direct |
| 7.3.3 | Verify software is secured | `source-authenticity`, `build-provenance`, `release-traceability` | Direct |
| 7.3.5 | Software update integrity | `provenance-authenticity`, `dependency-completeness` | Direct |
| 7.2.2.2(d) | Secret push protection | `secret-scanning-push-protection` | Partial |
| 7.2.2.5(c) | Admin branch enforcement | `branch-protection-admin-enforcement` | Partial |
| 7.2.2.2(e) | Dismiss stale reviews on push | `dismiss-stale-reviews-on-push` | Partial |
| 7.2.2.5(d) | Pinned CI/CD dependencies | `actions-pinned-dependencies` | Direct |
| 7.2.2.5(e) | Environment protection rules | `environment-protection-rules` | Partial |
| 7.2.2.2(f) | Code scanning alert resolution | `code-scanning-alerts-resolved` | Direct |
| 7.2.2.3.1 | Dependency license compliance | `dependency-license-compliance` | Partial |
| 7.3.4 | SBOM attestation | `sbom-attestation` | Direct |
| 7.3.6 | Release asset attestation | `release-asset-attestation` | Direct |
| 7.2.2.5(f) | Privileged workflow detection | `privileged-workflow-detection` | Direct |

### WP.29 Note

UN-R155 is a **vehicle type-approval regulation**. gh-verify provides evidence for Clause 7.2.2 (CSMS processes) and Clause 7.3 (vehicle type requirements related to software). Full WP.29 compliance requires additional organizational, operational, and post-production controls beyond the scope of SDLC verification.

For automotive SDLC verification, we recommend:
```bash
gh verify pr 42 --policy slsa-l3
gh verify release v1.0.0 --policy slsa-l3
gh verify repo --policy soc2
```

---

## ISO/SAE 21434 (Road Vehicles - Cybersecurity Engineering)

| Clause | Work Product | gh-verify Control(s) | Mapping |
|---|---|---|---|
| 10.4.1 | Implementation verification report | `test-coverage`, `required-status-checks` | Direct |
| 10.4.2 | Review of implementation | `review-independence`, `two-party-review`, `stale-review` | Direct |
| 11.4 | Component verification | `dependency-signature`, `dependency-provenance` | Direct |
| 13.3 | Configuration management | `branch-protection-enforcement`, `branch-history-integrity`, `merge-commit-policy` | Direct |
| 13.4 | Change management | `change-request-size`, `description-quality`, `issue-linkage` | Direct |

---

## Policy Preset Decision Mappings

Each preset maps control statuses (`violated`, `indeterminate`) to decisions (`fail`, `review`).
`satisfied` always maps to `pass`; `not_applicable` always maps to `pass`.

### default

All violations and indeterminate results are `fail`.

### oss

Relaxed for open-source workflows where unsigned commits and self-review are common.

| On violation → `review` | On indeterminate → `review` |
|---|---|
| `source-authenticity` | `review-independence` |
| `two-party-review` | `required-status-checks` |
| `branch-protection-enforcement` | `branch-history-integrity` |
| `issue-linkage` | `branch-protection-enforcement` |
| `conventional-title` | `two-party-review` |
| `codeowners-coverage` | `codeowners-coverage` |
| `vulnerability-scanning` | `vulnerability-scanning` |
| `secret-scanning` | `secret-scanning` |

All other violations and indeterminate results are `fail`.

### aiops

All violations are `fail`. All indeterminate results are `review`.

### soc1

| On violation → `review` | On indeterminate → `review` |
|---|---|
| `change-request-size` | `change-request-size` |
| `scoped-change` | `scoped-change` |
| `description-quality` | `description-quality` |
| `merge-commit-policy` | `merge-commit-policy` |
| `conventional-title` | `conventional-title` |
| `test-coverage` | `test-coverage` |
| `source-authenticity` | `source-authenticity` |

All other violations and indeterminate results are `fail`.

### soc2

| On violation → `review` | On indeterminate → `review` |
|---|---|
| Advisory: `change-request-size`, `scoped-change`, `description-quality`, `merge-commit-policy`, `conventional-title`, `issue-linkage` | Build: `build-provenance`, `hosted-build-platform`, `provenance-authenticity`, `build-isolation` |
| OSS-origin: `security-policy` | Dependency: `dependency-signature`, `dependency-provenance`, `dependency-signer-verified`, `dependency-completeness` |
| | Advisory + OSS-origin (same as violation list) |

All other violations and indeterminate results are `fail`.

### slsa-l1 through slsa-l4

All violations are `fail`. Indeterminate results for required controls are `fail`; others are `review`.

| Level | Required controls |
|---|---|
| L1 | `source-authenticity`, `review-independence`, `build-provenance`, `required-status-checks`, `dependency-signature` |
| L2 | L1 + `branch-history-integrity`, `hosted-build-platform`, `provenance-authenticity`, `dependency-provenance` |
| L3 | L2 + `branch-protection-enforcement`, `build-isolation`, `dependency-signer-verified` |
| L4 | L3 + `two-party-review`, `dependency-completeness` |

### ismap

| On violation → `review` (recommended) | On indeterminate → `review` |
|---|---|
| `change-request-size`, `scoped-change`, `description-quality`, `merge-commit-policy`, `conventional-title`, `issue-linkage`, `actions-pinned-dependencies`, `dependency-license-compliance`, `sbom-attestation`, `release-asset-attestation` | Build: `build-provenance`, `hosted-build-platform`, `provenance-authenticity`, `build-isolation` |
| | Dependency: `dependency-signature`, `dependency-provenance`, `dependency-signer-verified`, `dependency-completeness` |

All other violations and indeterminate results are `fail`.

### pci-dss

| On violation → `review` (advisory) | On indeterminate → `review` |
|---|---|
| `test-coverage`, `scoped-change`, `conventional-title`, `merge-commit-policy`, `dismiss-stale-reviews-on-push`, `sbom-attestation`, `release-asset-attestation` | Dependency: `dependency-signature`, `dependency-provenance` |

All other violations and indeterminate results are `fail`.

### tisax

| On violation → `review` (recommended) | On indeterminate → `review` |
|---|---|
| `test-coverage`, `scoped-change`, `conventional-title`, `merge-commit-policy`, `issue-linkage` | DevEnv: `build-isolation`, `hosted-build-platform` |

All other violations and indeterminate results are `fail`.

### nist-800-53

| On violation → `review` | On indeterminate → `review` |
|---|---|
| Audit: `issue-linkage`, `release-traceability` | Build: `build-provenance`, `hosted-build-platform`, `provenance-authenticity`, `build-isolation` |
| DevQuality: `change-request-size`, `scoped-change`, `description-quality`, `merge-commit-policy`, `conventional-title`, `dismiss-stale-reviews-on-push`, `dependency-license-compliance`, `sbom-attestation`, `release-asset-attestation` | Dependency: `dependency-signature`, `dependency-provenance`, `dependency-signer-verified`, `dependency-completeness` |

All other violations and indeterminate results are `fail`.

### wp29

| On violation → `review` (recommended) | On indeterminate → `review` |
|---|---|
| `change-request-size`, `description-quality`, `scoped-change`, `issue-linkage`, `stale-review`, `conventional-title`, `merge-commit-policy`, `two-party-review`, `codeowners-coverage`, `secret-scanning`, `secret-scanning-push-protection`, `branch-protection-admin-enforcement`, `dismiss-stale-reviews-on-push`, `environment-protection-rules`, `dependency-license-compliance` | DevEnv: `branch-protection-enforcement`, `build-isolation` |

All other violations and indeterminate results are `fail`.

---

## Coverage Summary

| Framework | Direct Mappings | Partial Mappings | Not Covered |
|---|---|---|---|
| NIST SP 800-53 | 22 | 3 | — |
| PCI DSS v4.0 | 13 | 7 | 6 (runtime/data) |
| ISMAP | 15 | 8 | 2 |
| TISAX | 17 | 4 | — |
| WP.29 / UN-R155 | 8 | 7 | — |
| ISO/SAE 21434 | 5 | 0 | — |

## Requesting New Mappings

If your compliance framework is not listed, you can:
1. Use OPA/Rego custom policies to encode your framework's requirements (see [custom-policies.md](custom-policies.md))
2. Open an issue at [libverify](https://github.com/HikaruEgashira/libverify/issues) requesting a new policy preset
