use crate::control::{Control, ControlFinding, ControlId};
use crate::evidence::{EvidenceBundle, EvidenceState};
use crate::integrity::branch_history_severity;
use crate::verdict::Severity;

/// Source L2: Verifies that branch history is continuous and protected from
/// force-push and deletion.
pub struct BranchHistoryIntegrityControl;

impl Control for BranchHistoryIntegrityControl {
    fn id(&self) -> ControlId {
        ControlId::BranchHistoryIntegrity
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

                let unprotected: Vec<&str> = value
                    .iter()
                    .filter(|r| !r.force_push_blocked || !r.deletion_blocked)
                    .map(|r| r.branch_pattern.as_str())
                    .collect();

                match branch_history_severity(unprotected.len()) {
                    Severity::Pass => vec![ControlFinding::satisfied(
                        id,
                        format!(
                            "All {} branch protection rule(s) block force-push and deletion",
                            value.len()
                        ),
                        subjects,
                    )],
                    _ => vec![ControlFinding::violated(
                        id,
                        format!(
                            "Branch(es) missing force-push or deletion protection: {}",
                            unprotected.join(", ")
                        ),
                        subjects,
                    )],
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::ControlStatus;
    use crate::evidence::{BranchProtectionEvidence, EvidenceGap};

    fn make_rule(
        pattern: &str,
        force_push_blocked: bool,
        deletion_blocked: bool,
    ) -> BranchProtectionEvidence {
        BranchProtectionEvidence {
            branch_pattern: pattern.to_string(),
            force_push_blocked,
            deletion_blocked,
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
        let findings = BranchHistoryIntegrityControl.evaluate(&evidence);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ControlStatus::NotApplicable);
        assert_eq!(findings[0].control_id, ControlId::BranchHistoryIntegrity);
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
        let findings = BranchHistoryIntegrityControl.evaluate(&evidence);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ControlStatus::Indeterminate);
        assert_eq!(findings[0].evidence_gaps.len(), 1);
    }

    #[test]
    fn satisfied_when_all_rules_block_force_push_and_deletion() {
        let evidence = EvidenceBundle {
            branch_protection: EvidenceState::complete(vec![
                make_rule("main", true, true),
                make_rule("release/*", true, true),
            ]),
            ..Default::default()
        };
        let findings = BranchHistoryIntegrityControl.evaluate(&evidence);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ControlStatus::Satisfied);
        assert_eq!(findings[0].subjects.len(), 2);
    }

    #[test]
    fn violated_when_force_push_allowed() {
        let evidence = EvidenceBundle {
            branch_protection: EvidenceState::complete(vec![make_rule("main", false, true)]),
            ..Default::default()
        };
        let findings = BranchHistoryIntegrityControl.evaluate(&evidence);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ControlStatus::Violated);
        assert!(findings[0].rationale.contains("main"));
    }

    #[test]
    fn violated_when_deletion_allowed() {
        let evidence = EvidenceBundle {
            branch_protection: EvidenceState::complete(vec![make_rule("main", true, false)]),
            ..Default::default()
        };
        let findings = BranchHistoryIntegrityControl.evaluate(&evidence);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ControlStatus::Violated);
        assert!(findings[0].rationale.contains("main"));
    }

    #[test]
    fn not_applicable_when_rule_list_empty() {
        let evidence = EvidenceBundle {
            branch_protection: EvidenceState::complete(vec![]),
            ..Default::default()
        };
        let findings = BranchHistoryIntegrityControl.evaluate(&evidence);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ControlStatus::NotApplicable);
    }

    #[test]
    fn violated_when_any_rule_lacks_protection() {
        let evidence = EvidenceBundle {
            branch_protection: EvidenceState::complete(vec![
                make_rule("main", true, true),
                make_rule("develop", false, false),
            ]),
            ..Default::default()
        };
        let findings = BranchHistoryIntegrityControl.evaluate(&evidence);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ControlStatus::Violated);
        assert!(findings[0].rationale.contains("develop"));
        assert!(!findings[0].rationale.contains("main"));
    }

    #[test]
    fn correct_control_id() {
        assert_eq!(
            BranchHistoryIntegrityControl.id(),
            ControlId::BranchHistoryIntegrity
        );
    }
}
