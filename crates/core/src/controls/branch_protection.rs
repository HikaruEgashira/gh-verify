use crate::control::{Control, ControlFinding, ControlId};
use crate::evidence::EvidenceBundle;

/// Verifies that the repository's default branch has adequate protection rules.
///
/// Required settings: `enforce_admins` and `dismiss_stale_reviews` must both be enabled.
pub struct BranchProtectionControl;

impl Control for BranchProtectionControl {
    fn id(&self) -> ControlId {
        ControlId::BranchProtection
    }

    fn evaluate(&self, evidence: &EvidenceBundle) -> Vec<ControlFinding> {
        let id = self.id();
        let subject = "repo_policy:repository".to_string();

        let policy = match evidence.repository_policy.value() {
            Some(p) => p,
            None => {
                return if matches!(
                    evidence.repository_policy,
                    crate::evidence::EvidenceState::NotApplicable
                ) {
                    vec![ControlFinding::not_applicable(
                        id,
                        "Repository policy evidence is not applicable",
                    )]
                } else {
                    vec![ControlFinding::indeterminate(
                        id,
                        "Repository policy evidence is missing",
                        vec![subject],
                        evidence.repository_policy.gaps().to_vec(),
                    )]
                };
            }
        };

        let config = match policy.branch_protection.value() {
            Some(c) => c,
            None => {
                return vec![ControlFinding::indeterminate(
                    id,
                    "branch protection not configured or inaccessible",
                    vec![subject],
                    policy.branch_protection.gaps().to_vec(),
                )];
            }
        };

        let mut violations = Vec::new();
        if !config.enforce_admins {
            violations.push("enforce_admins is disabled");
        }
        if !config.dismiss_stale_reviews {
            violations.push("dismiss_stale_reviews is disabled");
        }

        if violations.is_empty() {
            vec![ControlFinding::satisfied(
                id,
                "Branch protection rules meet requirements: enforce_admins and dismiss_stale_reviews are enabled",
                vec![subject],
            )]
        } else {
            vec![ControlFinding::violated(
                id,
                format!(
                    "Branch protection is insufficient: {}",
                    violations.join(", ")
                ),
                vec![subject],
            )]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::ControlStatus;
    use crate::evidence::{
        BranchProtectionConfig, EvidenceBundle, EvidenceGap, EvidenceState, RepositoryPolicy,
    };

    fn make_config(enforce_admins: bool, dismiss_stale_reviews: bool) -> BranchProtectionConfig {
        BranchProtectionConfig {
            required_reviews: 1,
            dismiss_stale_reviews,
            require_code_owner_reviews: false,
            enforce_admins,
            required_signatures: false,
        }
    }

    fn make_bundle(policy_state: EvidenceState<RepositoryPolicy>) -> EvidenceBundle {
        EvidenceBundle {
            repository_policy: policy_state,
            ..Default::default()
        }
    }

    fn make_policy(
        bp_state: EvidenceState<BranchProtectionConfig>,
    ) -> EvidenceState<RepositoryPolicy> {
        EvidenceState::complete(RepositoryPolicy {
            branch_protection: bp_state,
            required_status_checks: EvidenceState::not_applicable(),
        })
    }

    // --- Satisfied ---

    #[test]
    fn both_settings_enabled_is_satisfied() {
        let bundle = make_bundle(make_policy(EvidenceState::complete(make_config(
            true, true,
        ))));
        let findings = BranchProtectionControl.evaluate(&bundle);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ControlStatus::Satisfied);
        assert_eq!(findings[0].control_id, ControlId::BranchProtection);
        assert_eq!(findings[0].subjects, vec!["repo_policy:repository"]);
    }

    // --- Violated ---

    #[test]
    fn enforce_admins_disabled_is_violated() {
        let bundle = make_bundle(make_policy(EvidenceState::complete(make_config(
            false, true,
        ))));
        let findings = BranchProtectionControl.evaluate(&bundle);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ControlStatus::Violated);
        assert!(findings[0].rationale.contains("enforce_admins"));
    }

    #[test]
    fn dismiss_stale_reviews_disabled_is_violated() {
        let bundle = make_bundle(make_policy(EvidenceState::complete(make_config(
            true, false,
        ))));
        let findings = BranchProtectionControl.evaluate(&bundle);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ControlStatus::Violated);
        assert!(findings[0].rationale.contains("dismiss_stale_reviews"));
    }

    #[test]
    fn both_disabled_mentions_both_in_rationale() {
        let bundle = make_bundle(make_policy(EvidenceState::complete(make_config(
            false, false,
        ))));
        let findings = BranchProtectionControl.evaluate(&bundle);
        assert_eq!(findings[0].status, ControlStatus::Violated);
        assert!(findings[0].rationale.contains("enforce_admins"));
        assert!(findings[0].rationale.contains("dismiss_stale_reviews"));
    }

    // --- Indeterminate ---

    #[test]
    fn missing_repository_policy_is_indeterminate() {
        let bundle = make_bundle(EvidenceState::missing(vec![
            EvidenceGap::CollectionFailed {
                source: "github".to_string(),
                subject: "repo".to_string(),
                detail: "403 Forbidden".to_string(),
            },
        ]));
        let findings = BranchProtectionControl.evaluate(&bundle);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ControlStatus::Indeterminate);
        assert_eq!(findings[0].evidence_gaps.len(), 1);
    }

    #[test]
    fn branch_protection_not_applicable_is_indeterminate() {
        let bundle = make_bundle(make_policy(EvidenceState::not_applicable()));
        let findings = BranchProtectionControl.evaluate(&bundle);
        assert_eq!(findings[0].status, ControlStatus::Indeterminate);
        assert!(
            findings[0]
                .rationale
                .contains("not configured or inaccessible")
        );
    }

    #[test]
    fn branch_protection_missing_is_indeterminate() {
        let bundle = make_bundle(make_policy(EvidenceState::missing(vec![
            EvidenceGap::CollectionFailed {
                source: "github".to_string(),
                subject: "branch-protection".to_string(),
                detail: "not found".to_string(),
            },
        ])));
        let findings = BranchProtectionControl.evaluate(&bundle);
        assert_eq!(findings[0].status, ControlStatus::Indeterminate);
        assert_eq!(findings[0].evidence_gaps.len(), 1);
    }

    // --- NotApplicable ---

    #[test]
    fn not_applicable_repository_policy_is_not_applicable() {
        let bundle = make_bundle(EvidenceState::not_applicable());
        let findings = BranchProtectionControl.evaluate(&bundle);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ControlStatus::NotApplicable);
    }

    // --- Edge cases ---

    #[test]
    fn partial_policy_with_complete_branch_protection_is_evaluated() {
        let policy = EvidenceState::partial(
            RepositoryPolicy {
                branch_protection: EvidenceState::complete(make_config(true, true)),
                required_status_checks: EvidenceState::not_applicable(),
            },
            vec![EvidenceGap::Unsupported {
                source: "github".to_string(),
                capability: "status checks".to_string(),
            }],
        );
        let bundle = make_bundle(policy);
        let findings = BranchProtectionControl.evaluate(&bundle);
        assert_eq!(findings[0].status, ControlStatus::Satisfied);
    }

    #[test]
    fn control_id_is_branch_protection() {
        assert_eq!(BranchProtectionControl.id(), ControlId::BranchProtection);
    }
}
