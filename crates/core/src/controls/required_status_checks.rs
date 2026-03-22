use crate::control::{Control, ControlFinding, ControlId};
use crate::evidence::{EvidenceBundle, EvidenceState};

/// Verifies that the repository's default branch has at least one required status check.
pub struct RequiredStatusChecksControl;

impl Control for RequiredStatusChecksControl {
    fn id(&self) -> ControlId {
        ControlId::RequiredStatusChecks
    }

    fn evaluate(&self, evidence: &EvidenceBundle) -> Vec<ControlFinding> {
        let id = self.id();

        let checks = match &evidence.required_status_checks {
            EvidenceState::NotApplicable => {
                return vec![ControlFinding::not_applicable(
                    id,
                    "Required status checks evidence is not applicable",
                )];
            }
            EvidenceState::Missing { gaps } => {
                return vec![ControlFinding::indeterminate(
                    id,
                    "Required status checks evidence is unavailable",
                    vec!["repository".to_string()],
                    gaps.clone(),
                )];
            }
            EvidenceState::Complete { value } => value,
            EvidenceState::Partial { value, .. } => value,
        };

        if !checks.is_empty() {
            vec![ControlFinding::satisfied(
                id,
                format!(
                    "{} required status check(s) configured: {}",
                    checks.len(),
                    checks.join(", ")
                ),
                vec!["repository".to_string()],
            )]
        } else {
            vec![ControlFinding::violated(
                id,
                "no required status checks configured",
                vec!["repository".to_string()],
            )]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::ControlStatus;
    use crate::evidence::EvidenceGap;

    fn make_bundle(checks: Vec<String>) -> EvidenceBundle {
        EvidenceBundle {
            required_status_checks: EvidenceState::complete(checks),
            ..Default::default()
        }
    }

    // --- Satisfied ---

    #[test]
    fn one_status_check_configured_is_satisfied() {
        let findings =
            RequiredStatusChecksControl.evaluate(&make_bundle(vec!["ci/build".to_string()]));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ControlStatus::Satisfied);
        assert_eq!(findings[0].subjects, vec!["repository"]);
        assert!(findings[0].rationale.contains("1 required status check(s)"));
        assert!(findings[0].rationale.contains("ci/build"));
    }

    #[test]
    fn multiple_status_checks_configured_is_satisfied() {
        let findings = RequiredStatusChecksControl.evaluate(&make_bundle(vec![
            "ci/build".to_string(),
            "ci/test".to_string(),
            "ci/lint".to_string(),
        ]));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ControlStatus::Satisfied);
        assert!(findings[0].rationale.contains("3 required status check(s)"));
    }

    // --- Violated ---

    #[test]
    fn no_status_checks_configured_is_violated() {
        let findings = RequiredStatusChecksControl.evaluate(&make_bundle(vec![]));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ControlStatus::Violated);
        assert!(findings[0]
            .rationale
            .contains("no required status checks configured"));
    }

    // --- NotApplicable ---

    #[test]
    fn not_applicable_when_evidence_not_applicable() {
        let bundle = EvidenceBundle {
            required_status_checks: EvidenceState::not_applicable(),
            ..Default::default()
        };
        let findings = RequiredStatusChecksControl.evaluate(&bundle);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ControlStatus::NotApplicable);
    }

    // --- Indeterminate ---

    #[test]
    fn indeterminate_when_evidence_missing() {
        let bundle = EvidenceBundle {
            required_status_checks: EvidenceState::missing(vec![
                EvidenceGap::CollectionFailed {
                    source: "github".to_string(),
                    subject: "repository".to_string(),
                    detail: "403 Forbidden".to_string(),
                },
            ]),
            ..Default::default()
        };
        let findings = RequiredStatusChecksControl.evaluate(&bundle);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ControlStatus::Indeterminate);
        assert_eq!(findings[0].evidence_gaps.len(), 1);
    }

    // --- Edge cases ---

    #[test]
    fn partial_evidence_still_evaluates() {
        let bundle = EvidenceBundle {
            required_status_checks: EvidenceState::partial(
                vec!["ci/test".to_string()],
                vec![EvidenceGap::Unsupported {
                    source: "github".to_string(),
                    capability: "check runs".to_string(),
                }],
            ),
            ..Default::default()
        };
        let findings = RequiredStatusChecksControl.evaluate(&bundle);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, ControlStatus::Satisfied);
    }

    #[test]
    fn control_id_is_required_status_checks() {
        assert_eq!(
            RequiredStatusChecksControl.id(),
            ControlId::RequiredStatusChecks
        );
    }
}
