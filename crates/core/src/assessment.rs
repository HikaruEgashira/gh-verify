use serde::{Deserialize, Serialize};

use crate::control::{Control, ControlFinding, evaluate_all};
use crate::controls;
use crate::evidence::EvidenceBundle;
use crate::profile::{
    ControlProfile, ProfileOutcome, SlsaComprehensiveProfile, SlsaFoundationProfile, apply_profile,
};

/// Complete assessment result combining raw control findings with profile-mapped outcomes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssessmentReport {
    pub profile_name: String,
    pub findings: Vec<ControlFinding>,
    pub outcomes: Vec<ProfileOutcome>,
}

/// Evaluates all controls against evidence and maps findings through a profile.
pub fn assess(
    evidence: &EvidenceBundle,
    controls: &[Box<dyn Control>],
    profile: &dyn ControlProfile,
) -> AssessmentReport {
    let findings = evaluate_all(controls, evidence);
    let outcomes = apply_profile(profile, &findings);

    AssessmentReport {
        profile_name: profile.name().to_string(),
        findings,
        outcomes,
    }
}

/// Convenience entry point that runs the SLSA foundation control set and profile.
pub fn assess_with_slsa_foundation(evidence: &EvidenceBundle) -> AssessmentReport {
    let controls = controls::slsa_foundation_controls();
    assess(evidence, &controls, &SlsaFoundationProfile)
}

/// Entry point that runs all controls (Source + Build + Repo) with the comprehensive profile.
pub fn assess_with_slsa_comprehensive(evidence: &EvidenceBundle) -> AssessmentReport {
    let controls = controls::slsa_comprehensive_controls();
    assess(evidence, &controls, &SlsaComprehensiveProfile)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::{ControlId, ControlStatus};
    use crate::evidence::{
        ApprovalDecision, ApprovalDisposition, AuthenticityEvidence, ChangeRequestId,
        EvidenceState, GovernedChange, SourceRevision,
    };
    use crate::profile::{FindingSeverity, GateDecision};

    #[test]
    fn slsa_foundation_assessment_runs_controls_and_profile() {
        let evidence = EvidenceBundle {
            change_requests: vec![GovernedChange {
                id: ChangeRequestId::new("github_pr", "owner/repo#12"),
                title: "feat: add assessment entrypoint".to_string(),
                summary: None,
                submitted_by: Some("author".to_string()),
                changed_assets: EvidenceState::complete(vec![]),
                approval_decisions: EvidenceState::complete(vec![ApprovalDecision {
                    actor: "author".to_string(),
                    disposition: ApprovalDisposition::Approved,
                    submitted_at: Some("2026-03-15T00:00:00Z".to_string()),
                }]),
                source_revisions: EvidenceState::complete(vec![SourceRevision {
                    id: "deadbeef".to_string(),
                    authored_by: Some("author".to_string()),
                    committed_at: Some("2026-03-15T00:00:00Z".to_string()),
                    merge: false,
                    authenticity: EvidenceState::complete(AuthenticityEvidence::new(
                        false,
                        Some("unsigned".to_string()),
                    )),
                }]),
                work_item_refs: EvidenceState::complete(vec![]),
            }],
            promotion_batches: vec![],
            ..Default::default()
        };

        let report = assess_with_slsa_foundation(&evidence);

        assert_eq!(report.profile_name, "slsa-foundation");
        assert!(report.findings.len() >= 2);
        assert!(report.outcomes.iter().any(|outcome| {
            outcome.control_id == ControlId::ReviewIndependence
                && outcome.decision == GateDecision::Fail
        }));
        assert!(report.outcomes.iter().any(|outcome| {
            outcome.control_id == ControlId::SourceAuthenticity
                && outcome.severity == FindingSeverity::Error
        }));
        assert!(report.findings.iter().any(|finding| {
            finding.control_id == ControlId::ReviewIndependence
                && finding.status == ControlStatus::Violated
        }));
    }
}

#[cfg(test)]
#[path = "tests/assessment_hardening.rs"]
mod assessment_hardening;
