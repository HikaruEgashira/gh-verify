use super::*;
use crate::control::ControlStatus;
use crate::evidence::{
    AuthenticityEvidence, ChangeRequestId, EvidenceBundle, EvidenceGap, EvidenceState,
    GovernedChange, SourceRevision,
};

fn make_change(verified: bool) -> GovernedChange {
    GovernedChange {
        id: ChangeRequestId::new("github_pr", "owner/repo#7"),
        title: "fix: sign commits".to_string(),
        summary: None,
        submitted_by: Some("author".to_string()),
        changed_assets: EvidenceState::complete(vec![]),
        approval_decisions: EvidenceState::complete(vec![]),
        source_revisions: EvidenceState::complete(vec![SourceRevision {
            id: "deadbeef".to_string(),
            authored_by: Some("author".to_string()),
            committed_at: Some("2026-03-15T00:00:00Z".to_string()),
            merge: false,
            authenticity: EvidenceState::complete(AuthenticityEvidence::new(
                verified,
                Some("gpg".to_string()),
            )),
        }]),
        work_item_refs: EvidenceState::complete(vec![]),
    }
}

#[test]
fn empty_evidence_is_not_applicable() {
    let findings = SourceAuthenticityControl.evaluate(&EvidenceBundle {
        change_requests: vec![],
        promotion_batches: vec![],
        ..Default::default()
    });
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].status, ControlStatus::NotApplicable);
}

#[test]
fn promotion_batch_evaluated() {
    // Kills: not iterating promotion_batches
    use crate::evidence::PromotionBatch;
    let findings = SourceAuthenticityControl.evaluate(&EvidenceBundle {
        change_requests: vec![],
        promotion_batches: vec![PromotionBatch {
            id: "release:v1".into(),
            source_revisions: EvidenceState::complete(vec![SourceRevision {
                id: "abc123".into(),
                authored_by: Some("author".into()),
                committed_at: None,
                merge: false,
                authenticity: EvidenceState::complete(
                    crate::evidence::AuthenticityEvidence::new(true, Some("gpg".into())),
                ),
            }]),
            linked_change_requests: EvidenceState::complete(vec![]),
        }],
        ..Default::default()
    });
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].status, ControlStatus::Satisfied);
}

#[test]
fn promotion_batch_unsigned_is_violated() {
    use crate::evidence::PromotionBatch;
    let findings = SourceAuthenticityControl.evaluate(&EvidenceBundle {
        change_requests: vec![],
        promotion_batches: vec![PromotionBatch {
            id: "release:v2".into(),
            source_revisions: EvidenceState::complete(vec![SourceRevision {
                id: "abc123".into(),
                authored_by: Some("author".into()),
                committed_at: None,
                merge: false,
                authenticity: EvidenceState::complete(
                    crate::evidence::AuthenticityEvidence::new(false, None),
                ),
            }]),
            linked_change_requests: EvidenceState::complete(vec![]),
        }],
        ..Default::default()
    });
    assert_eq!(findings[0].status, ControlStatus::Violated);
}

#[test]
fn missing_source_revisions_is_indeterminate() {
    let mut change = make_change(true);
    change.source_revisions = EvidenceState::missing(vec![]);
    let findings = SourceAuthenticityControl.evaluate(&EvidenceBundle {
        change_requests: vec![change],
        promotion_batches: vec![],
        ..Default::default()
    });
    assert_eq!(findings[0].status, ControlStatus::Indeterminate);
}

#[test]
fn partial_revisions_with_gaps_is_indeterminate() {
    let mut change = make_change(true);
    change.source_revisions = EvidenceState::partial(
        vec![SourceRevision {
            id: "abc".into(),
            authored_by: None,
            committed_at: None,
            merge: false,
            authenticity: EvidenceState::missing(vec![EvidenceGap::CollectionFailed {
                source: "github".into(),
                subject: "abc".into(),
                detail: "timeout".into(),
            }]),
        }],
        vec![EvidenceGap::Truncated {
            source: "api".into(),
            subject: "revisions".into(),
        }],
    );
    let findings = SourceAuthenticityControl.evaluate(&EvidenceBundle {
        change_requests: vec![change],
        promotion_batches: vec![],
        ..Default::default()
    });
    assert_eq!(findings[0].status, ControlStatus::Indeterminate);
}

#[test]
fn multiple_revisions_mixed_signed_unsigned() {
    // Kills: only checking first revision
    let mut change = make_change(true);
    change.source_revisions = EvidenceState::complete(vec![
        SourceRevision {
            id: "aaa".into(),
            authored_by: Some("alice".into()),
            committed_at: None,
            merge: false,
            authenticity: EvidenceState::complete(
                crate::evidence::AuthenticityEvidence::new(true, Some("gpg".into())),
            ),
        },
        SourceRevision {
            id: "bbb".into(),
            authored_by: Some("bob".into()),
            committed_at: None,
            merge: false,
            authenticity: EvidenceState::complete(
                crate::evidence::AuthenticityEvidence::new(false, None),
            ),
        },
    ]);
    let findings = SourceAuthenticityControl.evaluate(&EvidenceBundle {
        change_requests: vec![change],
        promotion_batches: vec![],
        ..Default::default()
    });
    assert_eq!(findings[0].status, ControlStatus::Violated);
}

#[test]
fn multiple_changes_produce_multiple_findings() {
    let findings = SourceAuthenticityControl.evaluate(&EvidenceBundle {
        change_requests: vec![make_change(true), make_change(false)],
        promotion_batches: vec![],
        ..Default::default()
    });
    assert_eq!(findings.len(), 2);
    assert_eq!(findings[0].status, ControlStatus::Satisfied);
    assert_eq!(findings[1].status, ControlStatus::Violated);
}
