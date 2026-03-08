use anyhow::{Context, Result};

use super::client::GitHubClient;
use super::types::{CompareCommit, CompareResponse, PullRequestSummary, Review, Tag};

const MAX_PAGES: usize = 10;

/// Fetch repository tags (reverse chronological).
pub fn get_tags(client: &GitHubClient, owner: &str, repo: &str) -> Result<Vec<Tag>> {
    let mut all_tags: Vec<Tag> = Vec::new();
    let mut current_path = format!("/repos/{owner}/{repo}/tags?per_page=100");

    for _ in 0..MAX_PAGES {
        let (body, next_path) = client.get_with_link(&current_path)?;
        let tags: Vec<Tag> = serde_json::from_str(&body).context("failed to parse tags")?;
        all_tags.extend(tags);

        match next_path {
            Some(next) => current_path = next,
            None => break,
        }
    }

    Ok(all_tags)
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

/// Fetch reviews for a PR.
pub fn get_pr_reviews(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    pr_number: u32,
) -> Result<Vec<Review>> {
    let path = format!("/repos/{owner}/{repo}/pulls/{pr_number}/reviews");
    let body = client.get(&path)?;
    serde_json::from_str(&body).context("failed to parse PR reviews")
}
