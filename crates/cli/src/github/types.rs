use serde::Deserialize;

/// GitHub API response type for PR changed files.
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
#[derive(Debug, Clone, Deserialize)]
pub struct PrMetadata {
    pub number: u32,
    pub title: String,
    pub body: Option<String>,
}

/// GitHub API response type for a tag.
#[derive(Debug, Clone, Deserialize)]
pub struct Tag {
    pub name: String,
    pub commit: TagCommit,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TagCommit {
    pub sha: String,
}

/// Commit verification info from GitHub API.
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
#[derive(Debug, Clone, Deserialize)]
pub struct CompareResponse {
    pub commits: Vec<CompareCommit>,
    pub total_commits: u32,
}

/// A pull request summary (from commits/{sha}/pulls).
#[derive(Debug, Clone, Deserialize)]
pub struct PullRequestSummary {
    pub number: u32,
    pub state: String,
    pub merged_at: Option<String>,
    pub user: PrUser,
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
}
