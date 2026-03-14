pub mod detect_unscoped_change;
pub mod engine;
pub mod verify_conventional_commit;
pub mod verify_release_integrity;

use anyhow::Result;
use gh_verify_core::verdict::RuleResult;

use crate::github::types::{CompareCommit, PrFile, PrMetadata, PullRequestSummary, Review};

/// Context payload for rule execution.
#[allow(dead_code)]
pub enum RuleContext {
    Pr {
        pr_files: Vec<PrFile>,
        pr_metadata: PrMetadata,
    },
    Release {
        base_tag: String,
        head_tag: String,
        commits: Vec<CompareCommit>,
        commit_prs: Vec<CommitPrAssociation>,
        pr_reviews: Vec<PrReviewSet>,
    },
}

/// Per-commit PR association for release context.
pub struct CommitPrAssociation {
    pub commit_sha: String,
    pub prs: Vec<PullRequestSummary>,
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
