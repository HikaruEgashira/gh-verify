use gh_verify_core::evidence::{
    ApprovalDecision, ApprovalDisposition, AuthenticityEvidence, ChangeRequestId, ChangedAsset,
    EvidenceBundle, EvidenceGap, EvidenceState, GovernedChange, PromotionBatch, SourceRevision,
    WorkItemRef,
};

use crate::github::types::{CompareCommit, PrCommit, PrFile, PrMetadata, PullRequestSummary, Review};

pub struct GitHubCommitPullAssociation {
    pub commit_sha: String,
    pub pull_requests: Vec<PullRequestSummary>,
}

pub fn build_pull_request_bundle(
    repo: &str,
    pr_number: u32,
    pr_metadata: &PrMetadata,
    pr_files: &[PrFile],
    pr_reviews: &[Review],
    pr_commits: &[PrCommit],
) -> EvidenceBundle {
    EvidenceBundle {
        change_requests: vec![map_pull_request_evidence(
            repo,
            pr_number,
            pr_metadata,
            pr_files,
            pr_reviews,
            pr_commits,
        )],
        promotion_batches: Vec::new(),
    }
}

pub fn build_release_bundle(
    repo: &str,
    base_tag: &str,
    head_tag: &str,
    commits: &[CompareCommit],
    commit_pulls: &[GitHubCommitPullAssociation],
) -> EvidenceBundle {
    EvidenceBundle {
        change_requests: Vec::new(),
        promotion_batches: vec![map_promotion_batch_evidence(
            repo,
            base_tag,
            head_tag,
            commits,
            commit_pulls,
        )],
    }
}

pub fn map_pull_request_evidence(
    repo: &str,
    pr_number: u32,
    pr_metadata: &PrMetadata,
    pr_files: &[PrFile],
    pr_reviews: &[Review],
    pr_commits: &[PrCommit],
) -> GovernedChange {
    let changed_assets = map_changed_assets(pr_files);
    let approval_decisions = EvidenceState::complete(
        pr_reviews
            .iter()
            .map(|review| ApprovalDecision {
                actor: review.user.login.clone(),
                disposition: map_review_state(&review.state),
                submitted_at: review.submitted_at.clone(),
            })
            .collect(),
    );

    let source_revisions = EvidenceState::partial(
        pr_commits
            .iter()
            .map(|commit| SourceRevision {
                id: commit.sha.clone(),
                authored_by: None,
                committed_at: commit
                    .commit
                    .committer
                    .as_ref()
                    .and_then(|committer| committer.date.clone()),
                merge: false,
                authenticity: EvidenceState::not_applicable(),
            })
            .collect(),
        vec![EvidenceGap::Unsupported {
            source: "github".to_string(),
            capability: "PR commit author login is not normalized in the current adapter"
                .to_string(),
        }],
    );

    let work_item_refs = EvidenceState::complete(
        gh_verify_core::linkage::extract_issue_references(
            pr_metadata.body.as_deref().unwrap_or(""),
            &[],
        )
        .into_iter()
        .map(|reference| WorkItemRef {
            system: map_issue_ref_kind(&reference.kind).to_string(),
            value: reference.value,
            url: None,
        })
        .collect(),
    );

    GovernedChange {
        id: ChangeRequestId::new("github_pr", format!("{repo}#{pr_number}")),
        title: pr_metadata.title.clone(),
        summary: pr_metadata.body.clone(),
        submitted_by: None,
        changed_assets,
        approval_decisions,
        source_revisions,
        work_item_refs,
    }
}

pub fn map_promotion_batch_evidence(
    repo: &str,
    base_tag: &str,
    head_tag: &str,
    commits: &[CompareCommit],
    commit_pulls: &[GitHubCommitPullAssociation],
) -> PromotionBatch {
    let linked_change_requests: Vec<ChangeRequestId> = commit_pulls
        .iter()
        .flat_map(|association| association.pull_requests.iter())
        .map(|pr| ChangeRequestId::new("github_pr", format!("{repo}#{}", pr.number)))
        .collect();

    PromotionBatch {
        id: format!("github_release:{repo}:{base_tag}..{head_tag}"),
        source_revisions: EvidenceState::complete(
            commits
                .iter()
                .map(|commit| SourceRevision {
                    id: commit.sha.clone(),
                    authored_by: commit.author.as_ref().map(|author| author.login.clone()),
                    committed_at: None,
                    merge: commit.parents.len() >= 2,
                    authenticity: EvidenceState::complete(AuthenticityEvidence::new(
                        commit.commit.verification.verified,
                        Some(commit.commit.verification.reason.clone()),
                    )),
                })
                .collect(),
        ),
        linked_change_requests: EvidenceState::complete(linked_change_requests),
    }
}

fn map_changed_assets(pr_files: &[PrFile]) -> EvidenceState<Vec<ChangedAsset>> {
    let assets: Vec<ChangedAsset> = pr_files
        .iter()
        .map(|file| ChangedAsset {
            path: file.filename.clone(),
            diff_available: file.patch.is_some(),
        })
        .collect();

    let gaps: Vec<EvidenceGap> = pr_files
        .iter()
        .filter(|file| file.patch.is_none())
        .map(|file| EvidenceGap::DiffUnavailable {
            subject: file.filename.clone(),
        })
        .collect();

    if gaps.is_empty() {
        EvidenceState::complete(assets)
    } else {
        EvidenceState::partial(assets, gaps)
    }
}

fn map_review_state(state: &str) -> ApprovalDisposition {
    match state {
        "APPROVED" => ApprovalDisposition::Approved,
        "CHANGES_REQUESTED" => ApprovalDisposition::Rejected,
        "COMMENTED" => ApprovalDisposition::Commented,
        "DISMISSED" => ApprovalDisposition::Dismissed,
        _ => ApprovalDisposition::Unknown,
    }
}

fn map_issue_ref_kind(kind: &gh_verify_core::linkage::IssueRefKind) -> &'static str {
    match kind {
        gh_verify_core::linkage::IssueRefKind::GitHubIssue => "github_issue",
        gh_verify_core::linkage::IssueRefKind::JiraTicket => "jira_ticket",
        gh_verify_core::linkage::IssueRefKind::Url => "url",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::types::{
        CommitParent, CommitVerification, CompareCommitInner, PrCommitAuthor, PrCommitInner, PrUser,
    };

    #[test]
    fn pull_request_mapping_marks_missing_patch_as_partial() {
        let evidence = map_pull_request_evidence(
            "owner/repo",
            42,
            &PrMetadata {
                number: 42,
                title: "feat: add abstraction layer".to_string(),
                body: Some("fixes #10".to_string()),
            },
            &[PrFile {
                filename: "src/lib.rs".to_string(),
                status: "modified".to_string(),
                additions: 10,
                deletions: 2,
                changes: 12,
                patch: None,
            }],
            &[Review {
                user: PrUser {
                    login: "reviewer".to_string(),
                },
                state: "APPROVED".to_string(),
                submitted_at: Some("2026-03-15T00:00:00Z".to_string()),
            }],
            &[crate::github::types::PrCommit {
                sha: "abc123".to_string(),
                commit: PrCommitInner {
                    committer: Some(PrCommitAuthor {
                        date: Some("2026-03-15T00:00:00Z".to_string()),
                    }),
                },
            }],
        );

        assert!(matches!(
            evidence.changed_assets,
            EvidenceState::Partial { .. }
        ));
        assert!(matches!(
            evidence.source_revisions,
            EvidenceState::Partial { .. }
        ));
    }

    #[test]
    fn promotion_batch_mapping_preserves_signature_state() {
        let batch = map_promotion_batch_evidence(
            "owner/repo",
            "v0.1.0",
            "v0.2.0",
            &[CompareCommit {
                sha: "deadbeef".to_string(),
                commit: CompareCommitInner {
                    message: "feat: ship control layer".to_string(),
                    verification: CommitVerification {
                        verified: false,
                        reason: "unsigned".to_string(),
                    },
                },
                author: None,
                parents: vec![CommitParent {
                    sha: "parent".to_string(),
                }],
            }],
            &[GitHubCommitPullAssociation {
                commit_sha: "deadbeef".to_string(),
                pull_requests: vec![],
            }],
        );

        let revisions = match &batch.source_revisions {
            EvidenceState::Complete { value } => value,
            _ => panic!("source revisions should be complete"),
        };
        assert_eq!(revisions.len(), 1);
        assert!(matches!(
            revisions[0].authenticity,
            EvidenceState::Complete { .. }
        ));
    }

    #[test]
    fn pull_request_bundle_uses_new_evidence_entrypoint() {
        let bundle = build_pull_request_bundle(
            "owner/repo",
            42,
            &PrMetadata {
                number: 42,
                title: "feat: add abstraction layer".to_string(),
                body: Some("fixes #10".to_string()),
            },
            &[],
            &[],
            &[],
        );

        assert_eq!(bundle.change_requests.len(), 1);
        assert!(bundle.promotion_batches.is_empty());
    }
}
