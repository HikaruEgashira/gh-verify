use serde::Deserialize;

/// GitHub API response type for PR changed files.
#[derive(Debug, Clone, Deserialize)]
pub struct PrFile {
    pub filename: String,
    pub patch: Option<String>,
}

/// GitHub API response type for PR metadata.
#[derive(Debug, Clone, Deserialize)]
pub struct PrMetadata {
    pub number: u32,
    pub title: String,
    pub body: Option<String>,
    pub user: Option<PrUser>,
}

/// Pull request summary from the list pulls endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct PullRequestListItem {
    pub number: u32,
    pub title: String,
    pub merged_at: Option<String>,
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

/// Parent commit reference from the GitHub API.
#[derive(Debug, Clone, Deserialize)]
pub struct CommitParent {
    pub sha: String,
}

/// A commit from the compare API.
#[derive(Debug, Clone, Deserialize)]
pub struct CompareCommit {
    pub sha: String,
    pub commit: CompareCommitInner,
    pub author: Option<CommitAuthor>,
    #[serde(default)]
    pub parents: Vec<CommitParent>,
}

/// Response from the compare API.
#[derive(Debug, Clone, Deserialize)]
pub struct CompareResponse {
    pub commits: Vec<CompareCommit>,
}

/// A pull request summary (from commits/{sha}/pulls).
#[derive(Debug, Clone, Deserialize)]
pub struct PullRequestSummary {
    pub number: u32,
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
    pub submitted_at: Option<String>,
}

/// A commit on a PR (from the pulls/{number}/commits endpoint).
#[derive(Debug, Clone, Deserialize)]
pub struct PrCommit {
    pub sha: String,
    pub commit: PrCommitInner,
    pub author: Option<PrUser>,
}

/// Inner commit data for a PR commit.
#[derive(Debug, Clone, Deserialize)]
pub struct PrCommitInner {
    pub committer: Option<PrCommitAuthor>,
    pub verification: Option<CommitVerification>,
}

/// Committer info with timestamp.
#[derive(Debug, Clone, Deserialize)]
pub struct PrCommitAuthor {
    pub date: Option<String>,
}

/// Branch protection response from GitHub API (subset for status checks).
#[derive(Debug, Clone, Deserialize)]
pub struct BranchProtectionResponse {
    pub required_status_checks: Option<RequiredStatusChecksResponse>,
}

/// Required status checks from the branch protection API.
#[derive(Debug, Clone, Deserialize)]
pub struct RequiredStatusChecksResponse {
    pub strict: bool,
    pub contexts: Vec<String>,
}
