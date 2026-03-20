use anyhow::{Context, Result};

use super::client::GitHubClient;
use super::types::BranchProtectionResponse;

/// Fetch branch protection settings for a given branch.
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

/// Fetch the default branch name for a repository.
pub fn get_default_branch(client: &GitHubClient, owner: &str, repo: &str) -> Result<String> {
    let path = format!("/repos/{owner}/{repo}");
    let body = client.get(&path).context("failed to fetch repo info")?;
    let info: serde_json::Value =
        serde_json::from_str(&body).context("failed to parse repo info")?;
    let branch = info["default_branch"]
        .as_str()
        .unwrap_or("main")
        .to_string();
    Ok(branch)
}
