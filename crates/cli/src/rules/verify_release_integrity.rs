use anyhow::Result;
use gh_verify_core::integrity::{self, Commit, CommitPrAssoc, PrWithReviews};
use gh_verify_core::verdict::RuleResult;

use super::{Rule, RuleContext};

pub struct VerifyReleaseIntegrity;

impl Rule for VerifyReleaseIntegrity {
    fn id(&self) -> &'static str {
        "verify-release-integrity"
    }

    fn run(&self, ctx: &RuleContext) -> Result<Vec<RuleResult>> {
        let (commits_raw, commit_prs, pr_reviews) = match ctx {
            RuleContext::Release {
                commits,
                commit_prs,
                pr_reviews,
                ..
            } => (commits, commit_prs, pr_reviews),
            RuleContext::Pr { .. } => return Ok(vec![]),
        };

        // Convert API types to core types
        let commits: Vec<Commit> = commits_raw
            .iter()
            .map(|c| Commit {
                sha: c.sha.clone(),
                message: c.commit.message.clone(),
                verified: c.commit.verification.verified,
                author_login: c.author.as_ref().map(|a| a.login.clone()),
                // GitHub Compare API always returns parents[].
                // Empty = root commit (0 parents), 2+ = merge commit.
                parent_count: Some(c.parents.len() as u8),
            })
            .collect();

        // Build PR review data
        let mut prs: Vec<PrWithReviews> = Vec::new();
        for pr_rev in pr_reviews {
            let mut commit_authors = Vec::new();
            for assoc in commit_prs {
                for pr in &assoc.pull_requests {
                    if pr.number == pr_rev.pr_number {
                        for c in commits_raw {
                            if c.sha == assoc.commit_sha {
                                if let Some(ref author) = c.author {
                                    if !commit_authors.contains(&author.login) {
                                        commit_authors.push(author.login.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let approvers: Vec<String> = pr_rev
                .reviews
                .iter()
                .filter(|r| r.state == "APPROVED")
                .map(|r| r.user.login.clone())
                .collect();

            prs.push(PrWithReviews {
                pr_number: pr_rev.pr_number,
                pr_author: pr_rev.pr_author.clone(),
                commit_authors,
                approvers,
            });
        }

        // Build commit-PR associations
        let assocs: Vec<CommitPrAssoc> = commit_prs
            .iter()
            .map(|a| {
                let is_merge = commits
                    .iter()
                    .find(|c| c.sha == a.commit_sha)
                    .map(|c| c.is_merge())
                    .unwrap_or(false);
                CommitPrAssoc {
                    commit_sha: a.commit_sha.clone(),
                    pr_numbers: a.pull_requests.iter().map(|p| p.number).collect(),
                    is_merge,
                }
            })
            .collect();

        Ok(integrity::verify_release_integrity(&commits, &prs, &assocs))
    }
}
