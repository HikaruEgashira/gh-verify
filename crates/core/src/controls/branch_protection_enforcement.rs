use crate::control::{Control, ControlFinding, ControlId};
use crate::evidence::{BranchProtectionEvidence, EvidenceBundle, EvidenceState};
use crate::integrity::branch_protection_enforcement_severity;
use crate::verdict::Severity;

/// Source L3: Verifies that continuous technical controls are enforced on
/// protected branches (required reviews, status checks, admin enforcement).
pub struct BranchProtectionEnforcementControl;

impl Control for BranchProtectionEnforcementControl {
    fn id(&self) -> ControlId {
        ControlId::BranchProtectionEnforcement
    }

    fn evaluate(&self, evidence: &EvidenceBundle) -> Vec<ControlFinding> {
        let id = self.id();

        match &evidence.branch_protection {
            EvidenceState::NotApplicable => {
                vec![ControlFinding::not_applicable(
                    id,
                    "Branch protection evidence does not apply to this context",
                )]
            }
            EvidenceState::Missing { gaps } => {
                vec![ControlFinding::indeterminate(
                    id,
                    "Branch protection evidence could not be collected",
                    Vec::new(),
                    gaps.clone(),
                )]
            }
            EvidenceState::Complete { value } | EvidenceState::Partial { value, .. } => {
                if value.is_empty() {
                    return vec![ControlFinding::not_applicable(
                        id,
                        "No branch protection rules were present",
                    )];
                }

                let subjects: Vec<String> =
                    value.iter().map(|r| r.branch_pattern.clone()).collect();

                let violations: Vec<String> =
                    value.iter().filter_map(describe_enforcement_gap).collect();

                match branch_protection_enforcement_severity(violations.len()) {
                    Severity::Pass => vec![ControlFinding::satisfied(
                        id,
                        format!(
                            "All {} branch protection rule(s) enforce reviews, status checks, and admin restrictions",
                            value.len()
                        ),
                        subjects,
                    )],
                    _ => vec![ControlFinding::violated(
                        id,
                        format!("Enforcement gaps: {}", violations.join("; ")),
                        subjects,
                    )],
                }
            }
        }
    }
}

/// Returns a human-readable description of enforcement gaps for a single rule,
/// or `None` if the rule is fully enforced.
fn describe_enforcement_gap(rule: &BranchProtectionEvidence) -> Option<String> {
    let mut gaps = Vec::new();

    if rule.required_approving_review_count < 1 {
        gaps.push("no required reviews");
    }
    if rule.required_status_checks.is_empty() {
        gaps.push("no required status checks");
    }
    if !rule.enforce_admins {
        gaps.push("admin enforcement disabled");
    }

    if gaps.is_empty() {
        None
    } else {
        Some(format!("{}: {}", rule.branch_pattern, gaps.join(", ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::ControlStatus;
    use crate::evidence::EvidenceGap;

    fn make_enforced_rule(pattern: &str) -> BranchProtectionEvidence {
        BranchProtectionEvidence {
            branch_pattern: pattern.to_string(),
            force_push_blocked: true,
            deletion_blocked: true,
            required_approving_review_count: 1,
            dismiss_stale_reviews: true,
            require_code_owner_reviews: false,
            require_linear_history: false,
            require_signed_commits: false,
            required_status_checks: vec!["ci/build".to_string()],
            enforce_admins: true,
        }
    }

    fn make_unenforced_rule(pattern: &str) -> BranchProtectionEvidence {
        BranchProtectionEvidence {
            branch_pattern: pattern.to_string(),
            force_push_blocked: true,
            deletion_blocked: true,
            required_approving_review_count: 0,
            dismiss_stale_reviews: false,
            require_code_owner_reviews: false,
            require_linear_history: false,
            require_signed_commits: false,
            required_status_checks: vec![],
            enforce_admins: false,
        }
    }

    #[test]
    fn not_applicable_when_evidence_not_applicable() {
        let evidence = EvidenceBundle {
            branch_protection: EvidenceState::not_applicable(),
            ..Default::default()
        };
        let findings = BranchProtectionEnforcementControl.evaluate(&evidence);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ControlStatus::NotApplicable);
        assert_eq!(
            findings[0].control_id,
            ControlId::BranchProtectionEnforcement
        );
    }

    #[test]
    fn indeterminate_when_evidence_missing() {
        let evidence = EvidenceBundle {
            branch_protection: EvidenceState::missing(vec![EvidenceGap::CollectionFailed {
                source: "github".to_string(),
                subject: "branch-protection".to_string(),
                detail: "API returned 403".to_string(),
            }]),
            ..Default::default()
        };
        let findings = BranchProtectionEnforcementControl.evaluate(&evidence);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ControlStatus::Indeterminate);
        assert_eq!(findings[0].evidence_gaps.len(), 1);
    }

    #[test]
    fn satisfied_when_all_rules_fully_enforced() {
        let evidence = EvidenceBundle {
            branch_protection: EvidenceState::complete(vec![
                make_enforced_rule("main"),
                make_enforced_rule("release/*"),
            ]),
            ..Default::default()
        };
        let findings = BranchProtectionEnforcementControl.evaluate(&evidence);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ControlStatus::Satisfied);
        assert_eq!(findings[0].subjects.len(), 2);
    }

    #[test]
    fn violated_when_no_required_reviews() {
        let mut rule = make_enforced_rule("main");
        rule.required_approving_review_count = 0;
        let evidence = EvidenceBundle {
            branch_protection: EvidenceState::complete(vec![rule]),
            ..Default::default()
        };
        let findings = BranchProtectionEnforcementControl.evaluate(&evidence);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ControlStatus::Violated);
        assert!(findings[0].rationale.contains("no required reviews"));
    }

    #[test]
    fn violated_when_no_status_checks() {
        let mut rule = make_enforced_rule("main");
        rule.required_status_checks = vec![];
        let evidence = EvidenceBundle {
            branch_protection: EvidenceState::complete(vec![rule]),
            ..Default::default()
        };
        let findings = BranchProtectionEnforcementControl.evaluate(&evidence);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ControlStatus::Violated);
        assert!(findings[0].rationale.contains("no required status checks"));
    }

    #[test]
    fn violated_when_admin_enforcement_disabled() {
        let mut rule = make_enforced_rule("main");
        rule.enforce_admins = false;
        let evidence = EvidenceBundle {
            branch_protection: EvidenceState::complete(vec![rule]),
            ..Default::default()
        };
        let findings = BranchProtectionEnforcementControl.evaluate(&evidence);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ControlStatus::Violated);
        assert!(findings[0].rationale.contains("admin enforcement disabled"));
    }

    #[test]
    fn violated_reports_all_gaps_for_unenforced_rule() {
        let evidence = EvidenceBundle {
            branch_protection: EvidenceState::complete(vec![make_unenforced_rule("main")]),
            ..Default::default()
        };
        let findings = BranchProtectionEnforcementControl.evaluate(&evidence);
        assert_eq!(findings[0].status, ControlStatus::Violated);
        let rationale = &findings[0].rationale;
        assert!(rationale.contains("no required reviews"));
        assert!(rationale.contains("no required status checks"));
        assert!(rationale.contains("admin enforcement disabled"));
    }

    #[test]
    fn not_applicable_when_rule_list_empty() {
        let evidence = EvidenceBundle {
            branch_protection: EvidenceState::complete(vec![]),
            ..Default::default()
        };
        let findings = BranchProtectionEnforcementControl.evaluate(&evidence);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ControlStatus::NotApplicable);
    }

    #[test]
    fn violated_when_any_rule_fails() {
        let evidence = EvidenceBundle {
            branch_protection: EvidenceState::complete(vec![
                make_enforced_rule("main"),
                make_unenforced_rule("develop"),
            ]),
            ..Default::default()
        };
        let findings = BranchProtectionEnforcementControl.evaluate(&evidence);
        assert_eq!(findings[0].status, ControlStatus::Violated);
        assert!(findings[0].rationale.contains("develop"));
    }

    #[test]
    fn correct_control_id() {
        assert_eq!(
            BranchProtectionEnforcementControl.id(),
            ControlId::BranchProtectionEnforcement
        );
    }
}
