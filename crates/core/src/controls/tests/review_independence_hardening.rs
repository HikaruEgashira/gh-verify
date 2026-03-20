use super::*;
use crate::evidence::{
    ApprovalDecision, ApprovalDisposition, AuthenticityEvidence, ChangeRequestId, EvidenceBundle,
    EvidenceState, GovernedChange, SourceRevision,
};

fn make_change() -> GovernedChange {
    GovernedChange {
        id: ChangeRequestId::new("github_pr", "owner/repo#1"),
        title: "feat: add evidence layer".to_string(),
        summary: None,
        submitted_by: Some("author".to_string()),
        changed_assets: EvidenceState::complete(vec![]),
        approval_decisions: EvidenceState::complete(vec![ApprovalDecision {
            actor: "reviewer".to_string(),
            disposition: ApprovalDisposition::Approved,
            submitted_at: Some("2026-03-15T00:00:00Z".to_string()),
        }]),
        source_revisions: EvidenceState::complete(vec![SourceRevision {
            id: "abc123".to_string(),
            authored_by: Some("author".to_string()),
            committed_at: Some("2026-03-14T00:00:00Z".to_string()),
            merge: false,
            authenticity: EvidenceState::complete(AuthenticityEvidence::new(
                true,
                Some("gpg".to_string()),
            )),
        }]),
        work_item_refs: EvidenceState::complete(vec![]),
    }
}

#[test]
fn empty_change_requests_is_not_applicable() {
    // Kills: removing is_empty() early return
    let findings = ReviewIndependenceControl.evaluate(&EvidenceBundle {
        change_requests: vec![],
        promotion_batches: vec![],
    });
    assert_eq!(findings.len(), 1);
    assert_eq!(
        findings[0].status,
        crate::control::ControlStatus::NotApplicable
    );
}

#[test]
fn control_id_is_review_independence() {
    // Kills: returning wrong ControlId
    assert_eq!(ReviewIndependenceControl.id(), crate::control::ControlId::ReviewIndependence);
}

#[test]
fn missing_approval_decisions_is_indeterminate() {
    // Kills: not handling Missing approval_decisions
    let mut change = make_change();
    change.approval_decisions = EvidenceState::missing(vec![]);
    let finding = evaluate_change(&change);
    assert_eq!(
        finding.status,
        crate::control::ControlStatus::Indeterminate
    );
}

#[test]
fn missing_source_revisions_is_indeterminate() {
    let mut change = make_change();
    change.source_revisions = EvidenceState::missing(vec![]);
    let finding = evaluate_change(&change);
    assert_eq!(
        finding.status,
        crate::control::ControlStatus::Indeterminate
    );
}

#[test]
fn missing_submitted_by_is_indeterminate() {
    // Kills: removing submitted_by None check
    let mut change = make_change();
    change.submitted_by = None;
    let finding = evaluate_change(&change);
    assert_eq!(
        finding.status,
        crate::control::ControlStatus::Indeterminate
    );
}

#[test]
fn commented_review_does_not_count_as_approval() {
    // Kills: not checking disposition == Approved
    let mut change = make_change();
    change.approval_decisions = EvidenceState::complete(vec![ApprovalDecision {
        actor: "reviewer".to_string(),
        disposition: ApprovalDisposition::Commented,
        submitted_at: None,
    }]);
    let finding = evaluate_change(&change);
    assert_eq!(finding.status, crate::control::ControlStatus::Violated);
}

#[test]
fn rejected_review_does_not_count_as_approval() {
    let mut change = make_change();
    change.approval_decisions = EvidenceState::complete(vec![ApprovalDecision {
        actor: "reviewer".to_string(),
        disposition: ApprovalDisposition::Rejected,
        submitted_at: None,
    }]);
    let finding = evaluate_change(&change);
    assert_eq!(finding.status, crate::control::ControlStatus::Violated);
}

#[test]
fn dismissed_review_does_not_count_as_approval() {
    let mut change = make_change();
    change.approval_decisions = EvidenceState::complete(vec![ApprovalDecision {
        actor: "reviewer".to_string(),
        disposition: ApprovalDisposition::Dismissed,
        submitted_at: None,
    }]);
    let finding = evaluate_change(&change);
    assert_eq!(finding.status, crate::control::ControlStatus::Violated);
}

#[test]
fn multiple_changes_produce_multiple_findings() {
    // Kills: only evaluating first change
    let bundle = EvidenceBundle {
        change_requests: vec![make_change(), make_change()],
        promotion_batches: vec![],
    };
    let findings = ReviewIndependenceControl.evaluate(&bundle);
    assert_eq!(findings.len(), 2);
}

#[test]
fn submitter_approving_own_pr_is_violated() {
    // Different commit author but submitter approves their own PR
    let mut change = make_change();
    change.source_revisions = EvidenceState::complete(vec![SourceRevision {
        id: "abc123".to_string(),
        authored_by: Some("someone_else".to_string()),
        committed_at: Some("2026-03-14T00:00:00Z".to_string()),
        merge: false,
        authenticity: EvidenceState::complete(AuthenticityEvidence::new(true, None)),
    }]);
    change.approval_decisions = EvidenceState::complete(vec![ApprovalDecision {
        actor: "author".to_string(), // same as submitted_by
        disposition: ApprovalDisposition::Approved,
        submitted_at: None,
    }]);
    let finding = evaluate_change(&change);
    assert_eq!(finding.status, crate::control::ControlStatus::Violated);
}

#[test]
fn finding_subject_contains_change_id() {
    // Kills: not including subject in finding
    let finding = evaluate_change(&make_change());
    assert!(
        !finding.subjects.is_empty(),
        "finding should have at least one subject"
    );
    assert!(
        finding.subjects[0].contains("owner/repo"),
        "subject should contain change ID"
    );
}
