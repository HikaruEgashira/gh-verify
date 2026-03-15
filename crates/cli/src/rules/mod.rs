pub mod detect_missing_test;
pub mod detect_stale_approval;
pub mod detect_unscoped_change;
pub mod engine;
pub mod verify_branch_protection;
pub mod verify_conventional_commit;
pub mod verify_issue_linkage;
pub mod verify_pr_size;
pub mod verify_release_integrity;

use anyhow::Result;
use gh_verify_core::verdict::RuleResult;

use crate::adapters::github::GitHubCommitPullAssociation;
use crate::github::types::{
    CompareCommit, PrCommit, PrFile, PrMetadata, Review,
};

#[derive(Debug, Clone)]
pub struct PrRuleOptions {
    pub detect_missing_test: bool,
    pub test_patterns: Vec<String>,
    pub coverage_report: Option<String>,
}

impl Default for PrRuleOptions {
    fn default() -> Self {
        Self {
            detect_missing_test: true,
            test_patterns: vec![],
            coverage_report: None,
        }
    }
}

impl PrRuleOptions {
    pub fn for_benchmark() -> Self {
        Self {
            detect_missing_test: false,
            test_patterns: vec![],
            coverage_report: None,
        }
    }
}

/// Context payload for rule execution.
#[allow(dead_code)]
pub enum RuleContext {
    Pr {
        pr_files: Vec<PrFile>,
        pr_metadata: PrMetadata,
        pr_reviews: Vec<Review>,
        pr_commits: Vec<PrCommit>,
        options: PrRuleOptions,
    },
    Release {
        base_tag: String,
        head_tag: String,
        commits: Vec<CompareCommit>,
        commit_prs: Vec<GitHubCommitPullAssociation>,
        pr_reviews: Vec<PrReviewSet>,
    },
}

/// Per-PR review set for release context.
pub struct PrReviewSet {
    pub pr_number: u32,
    pub pr_author: String,
    pub reviews: Vec<Review>,
}

/// Trait for all rules.
pub trait Rule {
    fn id(&self) -> &'static str;
    fn run(&self, ctx: &RuleContext) -> Result<Vec<RuleResult>>;
}
