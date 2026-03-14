use anyhow::Result;
use gh_verify_core::branch_protection::{self, PrBranchInfo};
use gh_verify_core::verdict::RuleResult;

use super::{Rule, RuleContext};

pub struct VerifyBranchProtection;

impl Rule for VerifyBranchProtection {
    fn id(&self) -> &'static str {
        "verify-branch-protection"
    }

    fn run(&self, ctx: &RuleContext) -> Result<Vec<RuleResult>> {
        let (commit_prs, pr_reviews) = match ctx {
            RuleContext::Release {
                commit_prs,
                pr_reviews,
                ..
            } => (commit_prs, pr_reviews),
            RuleContext::Pr { .. } => return Ok(vec![]),
        };

        // Build PrBranchInfo from release context.
        // Deduplicate PRs by number (a PR may appear in multiple commit associations).
        let mut seen: Vec<u32> = Vec::new();
        let mut prs: Vec<PrBranchInfo> = Vec::new();

        for assoc in commit_prs {
            for pr_summary in &assoc.prs {
                if seen.contains(&pr_summary.number) {
                    continue;
                }
                seen.push(pr_summary.number);

                let review_count = pr_reviews
                    .iter()
                    .find(|r| r.pr_number == pr_summary.number)
                    .map(|r| {
                        r.reviews
                            .iter()
                            .filter(|rev| rev.state == "APPROVED")
                            .count() as u32
                    })
                    .unwrap_or(0);

                let base_ref = pr_summary
                    .base
                    .as_ref()
                    .map(|b| b.ref_name.clone())
                    .unwrap_or_default();

                prs.push(PrBranchInfo {
                    number: pr_summary.number,
                    base_ref,
                    review_count,
                    merged: pr_summary.merged_at.is_some(),
                });
            }
        }

        // Use default protected branches (main, master).
        // A future enhancement could read this from repository settings.
        Ok(branch_protection::check_branch_protection(&prs, &[]))
    }
}
