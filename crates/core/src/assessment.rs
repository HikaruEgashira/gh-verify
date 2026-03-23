use serde::{Deserialize, Serialize};

use crate::control::{Control, ControlFinding, ControlStatus, evaluate_all};
use crate::controls;
use crate::evidence::EvidenceBundle;
use crate::profile::{ControlProfile, ProfileOutcome, SlsaLevelProfile, apply_profile};
use crate::slsa::SlsaLevel;

/// Complete assessment result combining raw control findings with profile-mapped outcomes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssessmentReport {
    pub profile_name: String,
    pub findings: Vec<ControlFinding>,
    pub outcomes: Vec<ProfileOutcome>,
}

/// Assessment report with optional raw evidence bundle for audit trails.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationResult {
    #[serde(flatten)]
    pub report: AssessmentReport,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<EvidenceBundle>,
}

impl VerificationResult {
    pub fn new(report: AssessmentReport, evidence: Option<EvidenceBundle>) -> Self {
        Self { report, evidence }
    }
}

/// Evaluates all controls against evidence and maps findings through a profile.
pub fn assess(
    evidence: &EvidenceBundle,
    controls: &[Box<dyn Control>],
    profile: &dyn ControlProfile,
) -> AssessmentReport {
    let findings: Vec<ControlFinding> = evaluate_all(controls, evidence)
        .into_iter()
        .filter(|f| f.status != ControlStatus::NotApplicable)
        .collect();
    let outcomes = apply_profile(profile, &findings);

    AssessmentReport {
        profile_name: profile.name().to_string(),
        findings,
        outcomes,
    }
}

/// Assess at specific SLSA levels. Runs SLSA controls for both tracks;
/// the level-aware profile determines which are required vs advisory.
pub fn assess_with_slsa_levels(
    evidence: &EvidenceBundle,
    source_level: SlsaLevel,
    build_level: SlsaLevel,
) -> AssessmentReport {
    let controls = controls::slsa_controls(source_level, build_level);
    let profile = SlsaLevelProfile::new(source_level, build_level);
    assess(evidence, &controls, &profile)
}

/// Assess with all controls (SLSA + compliance) at specific SLSA levels.
pub fn assess_all_controls_with_levels(
    evidence: &EvidenceBundle,
    source_level: SlsaLevel,
    build_level: SlsaLevel,
) -> AssessmentReport {
    let mut controls = controls::slsa_controls(source_level, build_level);
    controls.extend(controls::compliance_controls());
    let profile = SlsaLevelProfile::new(source_level, build_level);
    assess(evidence, &controls, &profile)
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
    fn l1_assessment_runs_controls_and_profile() {
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

        let report = assess_with_slsa_levels(&evidence, SlsaLevel::L1, SlsaLevel::L1);

        assert_eq!(report.profile_name, "slsa-source-l1-build-l1");
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
