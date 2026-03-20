use crate::control::{Control, ControlFinding, ControlId};
use crate::evidence::{EvidenceBundle, EvidenceState};

/// Verifies that the repository's default branch requires a minimum number of reviewers.
pub struct RequiredReviewersControl;

impl Control for RequiredReviewersControl {
    fn id(&self) -> ControlId {
        ControlId::RequiredReviewers
    }

    fn evaluate(&self, evidence: &EvidenceBundle) -> Vec<ControlFinding> {
        let id = self.id();

        let policy = match &evidence.repository_policy {
            EvidenceState::NotApplicable => {
                return vec![ControlFinding::not_applicable(
                    id,
                    "Repository policy evidence is not applicable",
                )];
            }
            EvidenceState::Missing { gaps } => {
                return vec![ControlFinding::indeterminate(
                    id,
                    "Repository policy evidence is unavailable",
                    vec!["repository".to_string()],
                    gaps.clone(),
                )];
            }
            EvidenceState::Complete { value } => value,
            EvidenceState::Partial { value, .. } => value,
        };

        let config = match &policy.branch_protection {
            EvidenceState::NotApplicable | EvidenceState::Missing { .. } => {
                let gaps = policy.branch_protection.gaps().to_vec();
                return vec![ControlFinding::indeterminate(
                    id,
                    "Branch protection configuration is unavailable",
                    vec!["repository".to_string()],
                    gaps,
                )];
            }
            EvidenceState::Complete { value } => value,
            EvidenceState::Partial { value, .. } => value,
        };

        if config.required_reviews >= 1 {
            vec![ControlFinding::satisfied(
                id,
                format!(
                    "{} reviewer(s) required on default branch",
                    config.required_reviews
                ),
                vec!["repository".to_string()],
            )]
        } else {
            vec![ControlFinding::violated(
                id,
                "no minimum reviewer requirement configured",
                vec!["repository".to_string()],
            )]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::ControlStatus;
    use crate::evidence::{BranchProtectionConfig, EvidenceGap, RepositoryPolicy};

    fn make_bundle(required_reviews: u32) -> EvidenceBundle {
        EvidenceBundle {
            repository_policy: EvidenceState::complete(RepositoryPolicy {
                branch_protection: EvidenceState::complete(BranchProtectionConfig {
                    required_reviews,
                    dismiss_stale_reviews: false,
                    require_code_owner_reviews: false,
                    enforce_admins: false,
                    required_signatures: false,
                }),
                required_status_checks: EvidenceState::not_applicable(),
            }),
            ..Default::default()
        }
    }

    #[test]
    fn one_reviewer_required_is_satisfied() {
        let findings = RequiredReviewersControl.evaluate(&make_bundle(1));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ControlStatus::Satisfied);
        assert_eq!(findings[0].subjects, vec!["repository"]);
        assert!(findings[0].rationale.contains("1 reviewer(s) required"));
    }

    #[test]
    fn two_reviewers_required_is_satisfied() {
        let findings = RequiredReviewersControl.evaluate(&make_bundle(2));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ControlStatus::Satisfied);
        assert!(findings[0].rationale.contains("2 reviewer(s) required"));
    }

    #[test]
    fn zero_reviewers_required_is_violated() {
        let findings = RequiredReviewersControl.evaluate(&make_bundle(0));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ControlStatus::Violated);
        assert!(
            findings[0]
                .rationale
                .contains("no minimum reviewer requirement configured")
        );
    }

    #[test]
    fn not_applicable_when_policy_not_applicable() {
        let bundle = EvidenceBundle {
            repository_policy: EvidenceState::not_applicable(),
            ..Default::default()
        };
        let findings = RequiredReviewersControl.evaluate(&bundle);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ControlStatus::NotApplicable);
    }

    #[test]
    fn indeterminate_when_policy_missing() {
        let bundle = EvidenceBundle {
            repository_policy: EvidenceState::missing(vec![EvidenceGap::CollectionFailed {
                source: "github".to_string(),
                subject: "repository".to_string(),
                detail: "403 Forbidden".to_string(),
            }]),
            ..Default::default()
        };
        let findings = RequiredReviewersControl.evaluate(&bundle);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ControlStatus::Indeterminate);
        assert_eq!(findings[0].evidence_gaps.len(), 1);
    }

    #[test]
    fn indeterminate_when_branch_protection_not_applicable() {
        let bundle = EvidenceBundle {
            repository_policy: EvidenceState::complete(RepositoryPolicy {
                branch_protection: EvidenceState::not_applicable(),
                required_status_checks: EvidenceState::not_applicable(),
            }),
            ..Default::default()
        };
        let findings = RequiredReviewersControl.evaluate(&bundle);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ControlStatus::Indeterminate);
    }

    #[test]
    fn indeterminate_when_branch_protection_missing() {
        let bundle = EvidenceBundle {
            repository_policy: EvidenceState::complete(RepositoryPolicy {
                branch_protection: EvidenceState::missing(vec![EvidenceGap::CollectionFailed {
                    source: "github".to_string(),
                    subject: "repository".to_string(),
                    detail: "branch not found".to_string(),
                }]),
                required_status_checks: EvidenceState::not_applicable(),
            }),
            ..Default::default()
        };
        let findings = RequiredReviewersControl.evaluate(&bundle);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ControlStatus::Indeterminate);
        assert_eq!(findings[0].evidence_gaps.len(), 1);
    }

    #[test]
    fn partial_policy_still_evaluates() {
        let bundle = EvidenceBundle {
            repository_policy: EvidenceState::partial(
                RepositoryPolicy {
                    branch_protection: EvidenceState::complete(BranchProtectionConfig {
                        required_reviews: 2,
                        dismiss_stale_reviews: true,
                        require_code_owner_reviews: true,
                        enforce_admins: true,
                        required_signatures: true,
                    }),
                    required_status_checks: EvidenceState::not_applicable(),
                },
                vec![EvidenceGap::Unsupported {
                    source: "github".to_string(),
                    capability: "some optional field".to_string(),
                }],
            ),
            ..Default::default()
        };
        let findings = RequiredReviewersControl.evaluate(&bundle);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ControlStatus::Satisfied);
    }

    #[test]
    fn control_id_is_required_reviewers() {
        assert_eq!(RequiredReviewersControl.id(), ControlId::RequiredReviewers);
    }
}
