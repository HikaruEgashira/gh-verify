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

        // Extract APPROVED reviews with timestamps
        let approvals: Vec<ApprovalInfo> = reviews
            .iter()
            .filter(|r| r.state == "APPROVED")
            .filter_map(|r| {
                r.submitted_at.as_ref().map(|ts| ApprovalInfo {
                    submitted_at: ts.clone(),
                })
            })
            .collect();

        // Extract commit timestamps
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
