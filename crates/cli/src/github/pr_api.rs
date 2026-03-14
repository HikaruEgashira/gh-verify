use anyhow::{Context, Result};

use super::client::GitHubClient;
use super::types::{PrCommit, PrFile, PrMetadata, PullRequestListItem, Review};

const MAX_PAGES: usize = 10;

/// Fetch the list of changed files for a PR.
pub fn get_pr_files(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    pr_number: u32,
) -> Result<Vec<PrFile>> {
    let mut all_files: Vec<PrFile> = Vec::new();
    let mut current_path = format!("/repos/{owner}/{repo}/pulls/{pr_number}/files?per_page=100");

    for _ in 0..MAX_PAGES {
        let (body, next_path) = client.get_with_link(&current_path)?;
        let files: Vec<PrFile> = serde_json::from_str(&body).context("failed to parse PR files")?;
        all_files.extend(files);

        match next_path {
            Some(next) => current_path = next,
            None => break,
        }
    }

    Ok(all_files)
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
    let mut merged = Vec::new();
    let mut current_path = format!(
        "/repos/{owner}/{repo}/pulls?state=closed&sort=updated&direction=desc&per_page=100"
    );

    for _ in 0..MAX_PAGES {
        let (body, next_path) = client.get_with_link(&current_path)?;
        let prs: Vec<PullRequestListItem> =
            serde_json::from_str(&body).context("failed to parse pull request list")?;

        for pr in prs {
            if pr.merged_at.is_some() {
                merged.push(pr);
                if merged.len() >= limit {
                    return Ok(merged);
                }
            }
        }

        match next_path {
            Some(next) => current_path = next,
            None => break,
        }
    }

    Ok(merged)
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

/// Fetch commits for a PR.
pub fn get_pr_commits(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    pr_number: u32,
) -> Result<Vec<PrCommit>> {
    let mut all_commits: Vec<PrCommit> = Vec::new();
    let mut current_path =
        format!("/repos/{owner}/{repo}/pulls/{pr_number}/commits?per_page=100");

    for _ in 0..MAX_PAGES {
        let (body, next_path) = client.get_with_link(&current_path)?;
        let commits: Vec<PrCommit> =
            serde_json::from_str(&body).context("failed to parse PR commits")?;
        all_commits.extend(commits);

        match next_path {
            Some(next) => current_path = next,
            None => break,
        }
    }

    Ok(all_commits)
}
