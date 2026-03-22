use super::*;
use crate::control::ControlStatus;
use crate::evidence::{
    ApprovalDecision, ApprovalDisposition, AuthenticityEvidence, ChangeRequestId,
    EvidenceState, GovernedChange, SourceRevision,
};
use crate::profile::GateDecision;

#[test]
fn assessment_all_pass_scenario() {
    // Kills: only testing failure scenarios
    let evidence = EvidenceBundle {
        change_requests: vec![GovernedChange {
            id: ChangeRequestId::new("github_pr", "owner/repo#5"),
            title: "feat: all good".into(),
            summary: None,
            submitted_by: Some("author".into()),
            changed_assets: EvidenceState::complete(vec![]),
            approval_decisions: EvidenceState::complete(vec![ApprovalDecision {
                actor: "independent_reviewer".into(),
                disposition: ApprovalDisposition::Approved,
                submitted_at: Some("2026-03-15T00:00:00Z".into()),
            }]),
            source_revisions: EvidenceState::complete(vec![SourceRevision {
                id: "abc123".into(),
                authored_by: Some("author".into()),
                committed_at: Some("2026-03-15T00:00:00Z".into()),
                merge: false,
                authenticity: EvidenceState::complete(AuthenticityEvidence::new(
                    true,
                    Some("gpg".into()),
                )),
            }]),
            work_item_refs: EvidenceState::complete(vec![]),
        }],
        promotion_batches: vec![],
        ..Default::default()
    };

    let report = assess_with_slsa_foundation(&evidence);
    assert_eq!(report.profile_name, "slsa-foundation");
    assert!(report
        .outcomes
        .iter()
        .all(|o| o.decision == GateDecision::Pass));
    assert!(report
        .findings
        .iter()
        .all(|f| f.status == ControlStatus::Satisfied));
}

#[test]
fn assessment_empty_evidence() {
    // Kills: panicking on empty evidence
    // With empty evidence all controls return NotApplicable, which are filtered out.
    let evidence = EvidenceBundle::default();
    let report = assess_with_slsa_foundation(&evidence);
    assert!(report.findings.is_empty(), "empty evidence yields no applicable findings");
    assert_eq!(report.findings.len(), report.outcomes.len());
}

#[test]
fn assessment_findings_count_equals_outcomes_count() {
    // Kills: not mapping all findings to outcomes
    let evidence = EvidenceBundle {
        change_requests: vec![GovernedChange {
            id: ChangeRequestId::new("test", "1"),
            title: "t".into(),
            summary: None,
            submitted_by: Some("a".into()),
            changed_assets: EvidenceState::complete(vec![]),
            approval_decisions: EvidenceState::complete(vec![]),
            source_revisions: EvidenceState::complete(vec![]),
            work_item_refs: EvidenceState::complete(vec![]),
        }],
        promotion_batches: vec![],
        ..Default::default()
    };
    let report = assess_with_slsa_foundation(&evidence);
    assert_eq!(
        report.findings.len(),
        report.outcomes.len(),
        "every finding must map to an outcome"
    );
}
