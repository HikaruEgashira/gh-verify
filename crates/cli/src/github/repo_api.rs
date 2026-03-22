use anyhow::{Context, Result};
use serde::Deserialize;

use super::client::GitHubClient;

/// Fetch branch protection rules for a specific branch.
pub fn get_branch_protection(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    branch: &str,
) -> Result<BranchProtectionResponse> {
    let path = format!("/repos/{owner}/{repo}/branches/{branch}/protection");
    let body = client
        .get(&path)
        .context("failed to fetch branch protection")?;
    serde_json::from_str(&body).context("failed to parse branch protection response")
}

/// Response from GET /repos/{owner}/{repo}/branches/{branch}/protection.
#[derive(Debug, Deserialize)]
pub struct BranchProtectionResponse {
    pub required_status_checks: Option<RequiredStatusChecksConfig>,
    pub enforce_admins: Option<EnforceAdminsConfig>,
    pub required_pull_request_reviews: Option<RequiredPullRequestReviewsConfig>,
    pub allow_force_pushes: Option<AllowConfig>,
    pub allow_deletions: Option<AllowConfig>,
    pub required_linear_history: Option<RequiredConfig>,
    pub required_signatures: Option<RequiredConfig>,
}

#[derive(Debug, Deserialize)]
pub struct RequiredStatusChecksConfig {
    pub contexts: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct EnforceAdminsConfig {
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct RequiredPullRequestReviewsConfig {
    pub required_approving_review_count: Option<u32>,
    pub dismiss_stale_reviews: Option<bool>,
    pub require_code_owner_reviews: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct AllowConfig {
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct RequiredConfig {
    pub enabled: bool,
}
