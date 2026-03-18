use anyhow::{Context, Result};

use super::client::GitHubClient;
use super::types::{PrCommit, PrFile, PrMetadata, PullRequestListItem, Review};

/// Fetch the list of changed files for a PR.
pub fn get_pr_files(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    pr_number: u32,
) -> Result<Vec<PrFile>> {
    client.paginate(&format!(
        "/repos/{owner}/{repo}/pulls/{pr_number}/files?per_page=100"
    ))
}

/// Fetch PR metadata.
pub fn get_pr_metadata(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    pr_number: u32,
) -> Result<PrMetadata> {
    let path = format!("/repos/{owner}/{repo}/pulls/{pr_number}");
    let body = client.get(&path)?;
    serde_json::from_str(&body).context("failed to parse PR metadata")
}

/// Fetch recent merged PRs for a repository from the closed PR listing.
pub fn list_recent_merged_prs(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    limit: usize,
) -> Result<Vec<PullRequestListItem>> {
    let all: Vec<PullRequestListItem> = client.paginate(&format!(
        "/repos/{owner}/{repo}/pulls?state=closed&sort=updated&direction=desc&per_page=100"
    ))?;
    Ok(all
        .into_iter()
        .filter(|pr| pr.merged_at.is_some())
        .take(limit)
        .collect())
}

/// Fetch reviews for a PR (paginated).
pub fn get_pr_reviews(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    pr_number: u32,
) -> Result<Vec<Review>> {
    client.paginate(&format!(
        "/repos/{owner}/{repo}/pulls/{pr_number}/reviews?per_page=100"
    ))
}

/// Fetch commits for a PR.
pub fn get_pr_commits(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    pr_number: u32,
) -> Result<Vec<PrCommit>> {
    client.paginate(&format!(
        "/repos/{owner}/{repo}/pulls/{pr_number}/commits?per_page=100"
    ))
}
