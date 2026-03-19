use crate::control::{Control, ControlFinding, ControlId};
use crate::evidence::{
    ApprovalDisposition, EvidenceBundle, EvidenceGap, EvidenceState, GovernedChange,
};
use crate::integrity::is_approver_independent;

/// Verifies that at least one approver is independent from the change author and requester.
pub struct ReviewIndependenceControl;

impl Control for ReviewIndependenceControl {
    fn id(&self) -> ControlId {
        ControlId::ReviewIndependence
    }

    fn evaluate(&self, evidence: &EvidenceBundle) -> Vec<ControlFinding> {
        if evidence.change_requests.is_empty() {
            return vec![ControlFinding::not_applicable(
                self.id(),
                "No governed changes were supplied",
            )];
        }

        evidence
            .change_requests
            .iter()
            .map(evaluate_change)
            .collect()
    }
}

fn evaluate_change(change: &GovernedChange) -> ControlFinding {
    let subject = change.id.to_string();
    let mut gaps = collect_gaps(&change.approval_decisions);
    gaps.extend(collect_gaps(&change.source_revisions));

    let approvals = match change.approval_decisions.value() {
        Some(approvals) => approvals,
        None => {
            return ControlFinding::indeterminate(
                ControlId::ReviewIndependence,
                "Approval evidence is unavailable",
                vec![subject],
                gaps,
            );
        }
    };

    let revisions = match change.source_revisions.value() {
        Some(revisions) => revisions,
        None => {
            return ControlFinding::indeterminate(
                ControlId::ReviewIndependence,
                "Source revision evidence is unavailable",
                vec![subject],
                gaps,
            );
        }
    };

    let mut authors: Vec<&str> = revisions
        .iter()
        .filter_map(|revision| revision.authored_by.as_deref())
        .collect();
    authors.sort_unstable();
    authors.dedup();

    if change.submitted_by.is_none() {
        gaps.push(EvidenceGap::MissingField {
            source: "control-normalization".to_string(),
            subject: subject.clone(),
            field: "submitted_by".to_string(),
        });
    }

    if authors.is_empty() {
        gaps.push(EvidenceGap::MissingField {
            source: "control-normalization".to_string(),
            subject: subject.clone(),
            field: "source_revisions.authored_by".to_string(),
        });
    }

    if !gaps.is_empty() {
        return ControlFinding::indeterminate(
            ControlId::ReviewIndependence,
            "Independent review cannot be proven from partial evidence",
            vec![subject],
            gaps,
        );
    }

    let requester = change
        .submitted_by
        .as_deref()
        .expect("submitted_by guaranteed Some: early return on missing field");
    let has_independent_approval = approvals.iter().any(|approval| {
        if approval.disposition != ApprovalDisposition::Approved {
            return false;
        }
        let is_commit_author = authors.contains(&approval.actor.as_str());
        let is_pr_author = approval.actor == requester;
        is_approver_independent(is_commit_author, is_pr_author)
    });

    if has_independent_approval {
        ControlFinding::satisfied(
            ControlId::ReviewIndependence,
            "At least one approver is independent from both author and requester",
            vec![subject],
        )
    } else {
        ControlFinding::violated(
            ControlId::ReviewIndependence,
            "No independent approver was found for the change request",
            vec![subject],
        )
    }
}

fn collect_gaps<T>(state: &EvidenceState<T>) -> Vec<EvidenceGap> {
    state.gaps().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::{
        ApprovalDecision, AuthenticityEvidence, ChangeRequestId, EvidenceBundle, SourceRevision,
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
    fn independent_approval_is_satisfied() {
        let finding = evaluate_change(&make_change());
        assert_eq!(finding.status, crate::control::ControlStatus::Satisfied);
    }

    #[test]
    fn self_approval_is_violated() {
        let mut change = make_change();
        change.approval_decisions = EvidenceState::complete(vec![ApprovalDecision {
            actor: "author".to_string(),
            disposition: ApprovalDisposition::Approved,
            submitted_at: None,
        }]);

        let finding = evaluate_change(&change);
        assert_eq!(finding.status, crate::control::ControlStatus::Violated);
    }

    #[test]
    fn missing_authorship_is_indeterminate() {
        let mut change = make_change();
        change.source_revisions = EvidenceState::partial(
            vec![SourceRevision {
                id: "abc123".to_string(),
                authored_by: None,
                committed_at: Some("2026-03-14T00:00:00Z".to_string()),
                merge: false,
                authenticity: EvidenceState::not_applicable(),
            }],
            vec![EvidenceGap::Unsupported {
                source: "github".to_string(),
                capability: "author login unavailable for PR commit evidence".to_string(),
            }],
        );

        let findings = ReviewIndependenceControl.evaluate(&EvidenceBundle {
            change_requests: vec![change],
            promotion_batches: vec![],
        });

        assert_eq!(
            findings[0].status,
            crate::control::ControlStatus::Indeterminate
        );
    }

    // ================================================================
    // Mutation-hardening tests
    // ================================================================

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
}
