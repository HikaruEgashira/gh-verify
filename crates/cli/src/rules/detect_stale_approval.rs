use std::collections::HashMap;

use anyhow::Result;
use gh_verify_core::approval::{self, ApprovalInfo, ApprovalStatus, CommitTimestamp};
use gh_verify_core::verdict::{RuleResult, Severity};

use super::{Rule, RuleContext};

const RULE_ID: &str = "detect-stale-approval";

pub struct DetectStaleApproval;

impl Rule for DetectStaleApproval {
    fn id(&self) -> &'static str {
        RULE_ID
    }

    fn run(&self, ctx: &RuleContext) -> Result<Vec<RuleResult>> {
        let (reviews, commits) = match ctx {
            RuleContext::Pr {
                pr_reviews,
                pr_commits,
                ..
            } => (pr_reviews, pr_commits),
            RuleContext::Release { .. } => return Ok(vec![]),
        };

        // Deduplicate reviews per reviewer, keeping only the most recent.
        // If a reviewer's latest review is not APPROVED (e.g. DISMISSED or
        // CHANGES_REQUESTED), none of their earlier approvals count.
        let mut latest_by_user: HashMap<&str, (&str, &str)> = HashMap::new(); // login -> (state, submitted_at)
        for r in reviews {
            if let Some(ts) = r.submitted_at.as_deref() {
                let login = r.user.login.as_str();
                match latest_by_user.get(login) {
                    Some(&(_, prev_ts)) if prev_ts >= ts => {} // keep existing, it's newer
                    _ => {
                        latest_by_user.insert(login, (&r.state, ts));
                    }
                }
            }
        }

        // Only count reviewers whose most recent review is APPROVED
        let approvals: Vec<ApprovalInfo> = latest_by_user
            .values()
            .filter(|(state, _)| *state == "APPROVED")
            .map(|(_, ts)| ApprovalInfo {
                submitted_at: ts.to_string(),
            })
            .collect();

        // Extract commit timestamps.
        //
        // NOTE: We use `committer.date` from the git commit object, which can
        // differ from the actual push time. Git timestamps are mutable — they
        // change on rebase/amend — and GitHub's REST API does not expose push
        // timestamps. This means a force-pushed rebase that resets committer
        // dates could produce false negatives (approval appears fresh when the
        // code was actually re-pushed). This is a known GitHub API limitation.
        let commit_timestamps: Vec<CommitTimestamp> = commits
            .iter()
            .filter_map(|c| {
                let date = c.commit.committer.as_ref()?.date.as_ref()?;
                Some(CommitTimestamp {
                    sha: c.sha.clone(),
                    committed_at: date.clone(),
                })
            })
            .collect();

        let status = approval::classify_approval_status(&approvals, &commit_timestamps);

        let result = match status {
            ApprovalStatus::Stale => RuleResult {
                rule_id: RULE_ID.to_string(),
                severity: Severity::Error,
                message: "Approval is stale: commits were pushed after the last approval".to_string(),
                affected_files: vec![],
                suggestion: Some(
                    "Request a new review to cover the latest changes".to_string(),
                ),
            },
            ApprovalStatus::NoApproval => RuleResult {
                rule_id: RULE_ID.to_string(),
                severity: Severity::Warning,
                message: "No approvals found on this PR".to_string(),
                affected_files: vec![],
                suggestion: Some("Request a review before merging".to_string()),
            },
            ApprovalStatus::Fresh => RuleResult::pass(RULE_ID, "Approval covers the latest commit"),
            ApprovalStatus::NoCommits => RuleResult::pass(RULE_ID, "No commits found on this PR"),
        };

        Ok(vec![result])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::types::{PrCommit, PrCommitAuthor, PrCommitInner, PrMetadata, PrUser, Review};
    use crate::rules::RuleContext;

    fn make_review(login: &str, state: &str, submitted_at: &str) -> Review {
        Review {
            user: PrUser { login: login.to_string() },
            state: state.to_string(),
            submitted_at: Some(submitted_at.to_string()),
        }
    }

    fn make_commit(sha: &str, date: &str) -> PrCommit {
        PrCommit {
            sha: sha.to_string(),
            commit: PrCommitInner {
                committer: Some(PrCommitAuthor {
                    date: Some(date.to_string()),
                }),
            },
        }
    }

    fn make_ctx(reviews: Vec<Review>, commits: Vec<PrCommit>) -> RuleContext {
        RuleContext::Pr {
            pr_files: vec![],
            pr_metadata: PrMetadata {
                number: 1,
                title: "test".to_string(),
                body: None,
            },
            pr_reviews: reviews,
            pr_commits: commits,
            options: crate::rules::PrRuleOptions::default(),
        }
    }

    #[test]
    fn dismissed_approval_not_counted() {
        // Reviewer approves, then requests changes → approval should not count
        let reviews = vec![
            make_review("alice", "APPROVED", "2024-01-15T12:00:00Z"),
            make_review("alice", "CHANGES_REQUESTED", "2024-01-15T13:00:00Z"),
        ];
        let commits = vec![make_commit("aaa", "2024-01-15T11:00:00Z")];
        let ctx = make_ctx(reviews, commits);

        let rule = DetectStaleApproval;
        let results = rule.run(&ctx).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Warning);
        assert!(results[0].message.contains("No approvals"));
    }

    #[test]
    fn dismissed_then_reapproved_counts() {
        // Reviewer requests changes, then approves again → should count
        let reviews = vec![
            make_review("alice", "APPROVED", "2024-01-15T12:00:00Z"),
            make_review("alice", "CHANGES_REQUESTED", "2024-01-15T13:00:00Z"),
            make_review("alice", "APPROVED", "2024-01-15T14:00:00Z"),
        ];
        let commits = vec![make_commit("aaa", "2024-01-15T11:00:00Z")];
        let ctx = make_ctx(reviews, commits);

        let rule = DetectStaleApproval;
        let results = rule.run(&ctx).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Pass);
    }

    #[test]
    fn multiple_reviewers_one_dismissed() {
        // Alice approves then dismisses, Bob approves → only Bob's approval counts
        let reviews = vec![
            make_review("alice", "APPROVED", "2024-01-15T12:00:00Z"),
            make_review("alice", "DISMISSED", "2024-01-15T13:00:00Z"),
            make_review("bob", "APPROVED", "2024-01-15T14:00:00Z"),
        ];
        let commits = vec![make_commit("aaa", "2024-01-15T11:00:00Z")];
        let ctx = make_ctx(reviews, commits);

        let rule = DetectStaleApproval;
        let results = rule.run(&ctx).unwrap();
        assert_eq!(results.len(), 1);
        // Bob's approval at 14:00 covers the commit at 11:00
        assert_eq!(results[0].severity, Severity::Pass);
    }
}
