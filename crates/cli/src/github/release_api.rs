use anyhow::{Context, Result};

use super::client::GitHubClient;
use super::types::{CompareCommit, CompareResponse, PullRequestSummary, Tag};

/// Fetch repository tags (reverse chronological).
pub fn get_tags(client: &GitHubClient, owner: &str, repo: &str) -> Result<Vec<Tag>> {
    client.paginate(&format!("/repos/{owner}/{repo}/tags?per_page=100"))
}

/// Compare two refs and return commits between them.
pub fn compare_refs(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    base: &str,
    head: &str,
) -> Result<Vec<CompareCommit>> {
    let path = format!("/repos/{owner}/{repo}/compare/{base}...{head}");
    let body = client.get(&path)?;
    let response: CompareResponse =
        serde_json::from_str(&body).context("failed to parse compare response")?;
    Ok(response.commits)
}

/// Fetch PRs associated with a commit.
pub fn get_commit_pulls(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    sha: &str,
) -> Result<Vec<PullRequestSummary>> {
    let path = format!("/repos/{owner}/{repo}/commits/{sha}/pulls");
    let body = client.get(&path)?;
    serde_json::from_str(&body).context("failed to parse commit pulls")
}
