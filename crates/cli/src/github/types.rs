use serde::Deserialize;

/// GitHub API response type for PR changed files.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct PrFile {
    pub filename: String,
    pub status: String,
    pub additions: u32,
    pub deletions: u32,
    pub changes: u32,
    pub patch: Option<String>,
}

/// GitHub API response type for PR metadata.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct PrMetadata {
    pub number: u32,
    pub title: String,
    pub body: Option<String>,
}

/// Pull request summary from the list pulls endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct PullRequestListItem {
    pub number: u32,
    pub title: String,
    pub merged_at: Option<String>,
}

/// GitHub API response type for a tag.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct Tag {
    pub name: String,
    pub commit: TagCommit,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct TagCommit {
    pub sha: String,
}

/// Commit verification info from GitHub API.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct CommitVerification {
    pub verified: bool,
    pub reason: String,
}

/// Commit author info (top-level, optional).
#[derive(Debug, Clone, Deserialize)]
pub struct CommitAuthor {
    pub login: String,
}

/// Inner commit data.
#[derive(Debug, Clone, Deserialize)]
pub struct CompareCommitInner {
    pub message: String,
    pub verification: CommitVerification,
}

/// A commit from the compare API.
#[derive(Debug, Clone, Deserialize)]
pub struct CompareCommit {
    pub sha: String,
    pub commit: CompareCommitInner,
    pub author: Option<CommitAuthor>,
}

/// Response from the compare API.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct CompareResponse {
    pub commits: Vec<CompareCommit>,
    pub total_commits: u32,
}

/// A pull request summary (from commits/{sha}/pulls).
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct PullRequestSummary {
    pub number: u32,
    pub state: String,
    pub merged_at: Option<String>,
    pub user: PrUser,
    pub base: Option<PrBranchRef>,
}

/// Branch reference in a PR (base or head).
#[derive(Debug, Clone, Deserialize)]
pub struct PrBranchRef {
    #[serde(rename = "ref")]
    pub ref_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PrUser {
    pub login: String,
}

/// A PR review.
#[derive(Debug, Clone, Deserialize)]
pub struct Review {
    pub user: PrUser,
    pub state: String,
    pub submitted_at: Option<String>,
}

/// A commit on a PR (from the pulls/{number}/commits endpoint).
#[derive(Debug, Clone, Deserialize)]
pub struct PrCommit {
    pub sha: String,
    pub commit: PrCommitInner,
}

/// Inner commit data for a PR commit.
#[derive(Debug, Clone, Deserialize)]
pub struct PrCommitInner {
    pub committer: Option<PrCommitAuthor>,
}

/// Committer info with timestamp.
#[derive(Debug, Clone, Deserialize)]
pub struct PrCommitAuthor {
    pub date: Option<String>,
}
