use super::*;
use crate::control::ControlStatus;
use crate::evidence::{
    ApprovalDecision, ApprovalDisposition, AuthenticityEvidence, ChangeRequestId, EvidenceState,
    GovernedChange, SourceRevision,
};
use crate::profile::GateDecision;
use crate::slsa::SlsaLevel;

#[test]
fn assessment_all_pass_scenario() {
    let evidence = EvidenceBundle {
        change_requests: vec![GovernedChange {
            id: ChangeRequestId::new("github_pr", "owner/repo#5"),
            title: "feat: all good".into(),
            summary: None,
            submitted_by: Some("author".into()),
            changed_assets: EvidenceState::complete(vec![]),
            approval_decisions: EvidenceState::complete(vec![
                ApprovalDecision {
                    actor: "independent_reviewer_a".into(),
                    disposition: ApprovalDisposition::Approved,
                    submitted_at: Some("2026-03-15T00:00:00Z".into()),
                },
                ApprovalDecision {
                    actor: "independent_reviewer_b".into(),
                    disposition: ApprovalDisposition::Approved,
                    submitted_at: Some("2026-03-15T00:00:00Z".into()),
                },
            ]),
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

    let report = assess_with_slsa_levels(&evidence, SlsaLevel::L1, SlsaLevel::L1);
    assert_eq!(report.profile_name, "slsa-source-l1-build-l1");
    assert!(
        report
            .outcomes
            .iter()
            .all(|o| o.decision == GateDecision::Pass)
    );
    assert!(
        report
            .findings
            .iter()
            .all(|f| f.status == ControlStatus::Satisfied)
    );
}

#[test]
fn assessment_empty_evidence() {
    let evidence = EvidenceBundle::default();
    let report = assess_with_slsa_levels(&evidence, SlsaLevel::L1, SlsaLevel::L1);
    assert!(
        report.findings.is_empty(),
        "empty evidence yields no applicable findings"
    );
    assert_eq!(report.findings.len(), report.outcomes.len());
}

#[test]
fn assessment_findings_count_equals_outcomes_count() {
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
    let report = assess_with_slsa_levels(&evidence, SlsaLevel::L1, SlsaLevel::L1);
    assert_eq!(
        report.findings.len(),
        report.outcomes.len(),
        "every finding must map to an outcome"
    );
}
