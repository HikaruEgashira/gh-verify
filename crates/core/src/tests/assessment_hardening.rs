use super::*;
use crate::control::{ControlId, ControlStatus};
use crate::evidence::{
    ApprovalDecision, ApprovalDisposition, AuthenticityEvidence, ChangedAsset, ChangeRequestId,
    EvidenceState, GovernedChange, PromotionBatch, SourceRevision, WorkItemRef,
};
use crate::profile::{FindingSeverity, GateDecision};
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

#[test]
fn assess_all_controls_includes_compliance() {
    let evidence = EvidenceBundle {
        change_requests: vec![GovernedChange {
            id: ChangeRequestId::new("github_pr", "owner/repo#10"),
            title: "feat: add compliance controls".into(),
            summary: Some("This PR adds comprehensive compliance controls for SOC2.".into()),
            submitted_by: Some("author".into()),
            changed_assets: EvidenceState::complete(vec![ChangedAsset {
                path: "src/main.rs".into(),
                diff_available: true,
                additions: 10,
                deletions: 2,
                status: "modified".into(),
                diff: None,
            }]),
            approval_decisions: EvidenceState::complete(vec![ApprovalDecision {
                actor: "reviewer".into(),
                disposition: ApprovalDisposition::Approved,
                submitted_at: Some("2026-03-15T12:00:00Z".into()),
            }]),
            source_revisions: EvidenceState::complete(vec![SourceRevision {
                id: "abc123".into(),
                authored_by: Some("author".into()),
                committed_at: Some("2026-03-15T10:00:00Z".into()),
                merge: false,
                authenticity: EvidenceState::complete(AuthenticityEvidence::new(
                    true,
                    Some("gpg".into()),
                )),
            }]),
            work_item_refs: EvidenceState::complete(vec![WorkItemRef {
                system: "github_issue".into(),
                value: "#42".into(),
            }]),
        }],
        promotion_batches: vec![],
        ..Default::default()
    };

    let report = assess_all_controls_with_levels(&evidence, SlsaLevel::L1, SlsaLevel::L1);

    // All compliance controls should produce findings
    let compliance_ids = [
        ControlId::PrSize,
        ControlId::TestCoverage,
        ControlId::ScopedChange,
        ControlId::IssueLinkage,
        ControlId::StaleReview,
        ControlId::DescriptionQuality,
        ControlId::MergeCommitPolicy,
        ControlId::ConventionalTitle,
        ControlId::SecurityFileChange,
        // ReleaseTraceability is NotApplicable (no promotion_batches) → filtered out
    ];

    for id in &compliance_ids {
        assert!(
            report.findings.iter().any(|f| f.control_id == *id)
                || report.outcomes.iter().any(|o| o.control_id == *id),
            "compliance control {id:?} should produce a finding or outcome"
        );
    }

    // Compliance controls that are violated should map to Fail
    for outcome in report.outcomes.iter().filter(|o| compliance_ids.contains(&o.control_id)) {
        if outcome.severity == FindingSeverity::Error {
            assert_eq!(
                outcome.decision,
                GateDecision::Fail,
                "{:?} violated should fail gate",
                outcome.control_id
            );
        }
    }

    assert_eq!(
        report.findings.len(),
        report.outcomes.len(),
        "every finding must map to an outcome"
    );
}

#[test]
fn assess_all_controls_compliance_indeterminate_reviews() {
    // Compliance controls with Indeterminate should map to Review (advisory)
    let evidence = EvidenceBundle {
        change_requests: vec![GovernedChange {
            id: ChangeRequestId::new("github_pr", "owner/repo#20"),
            title: "feat: test".into(),
            summary: Some("A long enough description for quality.".into()),
            submitted_by: Some("author".into()),
            changed_assets: EvidenceState::missing(vec![]),
            approval_decisions: EvidenceState::missing(vec![]),
            source_revisions: EvidenceState::missing(vec![]),
            work_item_refs: EvidenceState::missing(vec![]),
        }],
        promotion_batches: vec![],
        ..Default::default()
    };

    let report = assess_all_controls_with_levels(&evidence, SlsaLevel::L1, SlsaLevel::L1);

    // Compliance controls with indeterminate findings should Review (not Fail)
    for outcome in &report.outcomes {
        if !crate::slsa::control_slsa_mapping(outcome.control_id).is_some() {
            // Compliance control
            if outcome.severity == FindingSeverity::Warning {
                assert_eq!(
                    outcome.decision,
                    GateDecision::Review,
                    "{:?} indeterminate compliance should review",
                    outcome.control_id
                );
            }
        }
    }
}

#[test]
fn assess_all_controls_release_traceability_with_batches() {
    let evidence = EvidenceBundle {
        change_requests: vec![],
        promotion_batches: vec![PromotionBatch {
            id: "github_release:owner/repo:v1.0..v2.0".into(),
            source_revisions: EvidenceState::complete(vec![SourceRevision {
                id: "abc123".into(),
                authored_by: Some("dev".into()),
                committed_at: None,
                merge: false,
                authenticity: EvidenceState::complete(AuthenticityEvidence::new(
                    true,
                    Some("gpg".into()),
                )),
            }]),
            linked_change_requests: EvidenceState::complete(vec![ChangeRequestId::new(
                "github_pr",
                "owner/repo#1",
            )]),
        }],
        ..Default::default()
    };

    let report = assess_all_controls_with_levels(&evidence, SlsaLevel::L1, SlsaLevel::L1);

    // ReleaseTraceability should be satisfied
    let traceability_finding = report
        .findings
        .iter()
        .find(|f| f.control_id == ControlId::ReleaseTraceability);
    assert!(
        traceability_finding.is_some(),
        "ReleaseTraceability should produce a finding for promotion batches"
    );
    assert_eq!(
        traceability_finding.unwrap().status,
        ControlStatus::Satisfied
    );
}
