use serde::{Deserialize, Serialize};

/// Status of the most recent approval relative to the most recent commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    /// Last commit is newer than the last approval — review is outdated.
    Stale,
    /// Last approval is at or after the last commit — review covers latest code.
    Fresh,
    /// No approvals exist on the PR.
    NoApproval,
    /// No commits found (empty PR).
    NoCommits,
}

/// A review approval with its submission timestamp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalInfo {
    pub submitted_at: String,
}

/// A commit with its timestamp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitTimestamp {
    pub sha: String,
    pub committed_at: String,
}

/// Compare two ISO 8601 timestamps.
/// Returns `true` if `last_approval_at < last_commit_at` (approval is stale).
///
/// Relies on ISO 8601 / RFC 3339 timestamps being lexicographically orderable
/// when in the same timezone (GitHub API always returns UTC with `Z` suffix).
pub fn is_approval_stale(last_approval_at: &str, last_commit_at: &str) -> bool {
    last_approval_at < last_commit_at
}

/// Classify the approval status of a PR given its approvals and commits.
pub fn classify_approval_status(
    approvals: &[ApprovalInfo],
    commits: &[CommitTimestamp],
) -> ApprovalStatus {
    if commits.is_empty() {
        return ApprovalStatus::NoCommits;
    }
    if approvals.is_empty() {
        return ApprovalStatus::NoApproval;
    }

    let last_approval = approvals
        .iter()
        .map(|a| a.submitted_at.as_str())
        .max()
        .expect("non-empty approvals");

    let last_commit = commits
        .iter()
        .map(|c| c.committed_at.as_str())
        .max()
        .expect("non-empty commits");

    if is_approval_stale(last_approval, last_commit) {
        ApprovalStatus::Stale
    } else {
        ApprovalStatus::Fresh
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 承認後のforce pushで四眼原則がバイパスされる攻撃パターンの検知
    #[test]
    fn stale_when_commit_after_approval() {
        let approvals = vec![ApprovalInfo {
            submitted_at: "2024-01-15T12:00:00Z".to_string(),
        }];
        let commits = vec![CommitTimestamp {
            sha: "abc123".to_string(),
            committed_at: "2024-01-15T13:00:00Z".to_string(),
        }];
        assert_eq!(
            classify_approval_status(&approvals, &commits),
            ApprovalStatus::Stale
        );
    }

    #[test]
    fn fresh_when_approval_after_commit() {
        let approvals = vec![ApprovalInfo {
            submitted_at: "2024-01-15T13:00:00Z".to_string(),
        }];
        let commits = vec![CommitTimestamp {
            sha: "abc123".to_string(),
            committed_at: "2024-01-15T12:00:00Z".to_string(),
        }];
        assert_eq!(
            classify_approval_status(&approvals, &commits),
            ApprovalStatus::Fresh
        );
    }

    #[test]
    fn no_approvals_returns_no_approval() {
        let commits = vec![CommitTimestamp {
            sha: "abc123".to_string(),
            committed_at: "2024-01-15T12:00:00Z".to_string(),
        }];
        assert_eq!(
            classify_approval_status(&[], &commits),
            ApprovalStatus::NoApproval
        );
    }

    #[test]
    fn no_commits_returns_no_commits() {
        let approvals = vec![ApprovalInfo {
            submitted_at: "2024-01-15T12:00:00Z".to_string(),
        }];
        assert_eq!(
            classify_approval_status(&approvals, &[]),
            ApprovalStatus::NoCommits
        );
    }

    /// 同一タイムスタンプは「承認がコミットをカバーしている」と見なす（GitHub APIのタイムスタンプ精度はsec単位）
    #[test]
    fn fresh_when_approval_at_same_time_as_commit() {
        let approvals = vec![ApprovalInfo {
            submitted_at: "2024-01-15T12:00:00Z".to_string(),
        }];
        let commits = vec![CommitTimestamp {
            sha: "abc123".to_string(),
            committed_at: "2024-01-15T12:00:00Z".to_string(),
        }];
        assert_eq!(
            classify_approval_status(&approvals, &commits),
            ApprovalStatus::Fresh
        );
    }

    #[test]
    fn uses_latest_approval_and_commit() {
        let approvals = vec![
            ApprovalInfo {
                submitted_at: "2024-01-15T10:00:00Z".to_string(),
            },
            ApprovalInfo {
                submitted_at: "2024-01-15T14:00:00Z".to_string(),
            },
        ];
        let commits = vec![
            CommitTimestamp {
                sha: "aaa".to_string(),
                committed_at: "2024-01-15T11:00:00Z".to_string(),
            },
            CommitTimestamp {
                sha: "bbb".to_string(),
                committed_at: "2024-01-15T13:00:00Z".to_string(),
            },
        ];
        assert_eq!(
            classify_approval_status(&approvals, &commits),
            ApprovalStatus::Fresh
        );
    }

    /// Property: classify_approval_status returns Stale iff last commit > last approval.
    #[test]
    fn stale_biconditional() {
        // Forward: commit after approval => Stale
        let approvals = vec![ApprovalInfo {
            submitted_at: "2024-01-15T12:00:00Z".to_string(),
        }];
        let commits = vec![CommitTimestamp {
            sha: "a".to_string(),
            committed_at: "2024-01-15T13:00:00Z".to_string(),
        }];
        assert_eq!(
            classify_approval_status(&approvals, &commits),
            ApprovalStatus::Stale
        );

        // Backward (contrapositive): approval after commit => not Stale
        let approvals2 = vec![ApprovalInfo {
            submitted_at: "2024-01-15T14:00:00Z".to_string(),
        }];
        assert_ne!(
            classify_approval_status(&approvals2, &commits),
            ApprovalStatus::Stale
        );
    }

    #[test]
    fn both_empty_returns_no_commits() {
        assert_eq!(
            classify_approval_status(&[], &[]),
            ApprovalStatus::NoCommits
        );
    }

    #[test]
    fn is_approval_stale_basic() {
        assert!(is_approval_stale(
            "2024-01-15T12:00:00Z",
            "2024-01-15T13:00:00Z"
        ));
        assert!(!is_approval_stale(
            "2024-01-15T13:00:00Z",
            "2024-01-15T12:00:00Z"
        ));
        assert!(!is_approval_stale(
            "2024-01-15T12:00:00Z",
            "2024-01-15T12:00:00Z"
        ));
    }
}
