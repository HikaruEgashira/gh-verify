# Compliance Framework Mapping

gh-verify's 28 SDLC controls map to multiple compliance frameworks.
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

## Coverage Summary

| Framework | Direct Mappings | Partial Mappings | Not Covered |
|---|---|---|---|
| NIST SP 800-53 | 15 | 2 | — |
| PCI DSS v4.0 | 6 | 4 | 6 (runtime/data) |
| ISMAP | 9 | 4 | 2 |
| TISAX | 7 | 3 | — |
| WP.29 / UN-R155 | 4 | 3 | — |
| ISO/SAE 21434 | 5 | 0 | — |

## Requesting New Mappings

If your compliance framework is not listed, you can:
1. Use OPA/Rego custom policies to encode your framework's requirements (see [custom-policies.md](custom-policies.md))
2. Open an issue at [libverify](https://github.com/HikaruEgashira/libverify/issues) requesting a new policy preset
