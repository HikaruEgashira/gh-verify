//! Conventional Commits compliance logic.
//!
//! Pure functions that check commit messages against the Conventional Commits
//! specification (<https://www.conventionalcommits.org/>).
//! No I/O, no unsafe.

use crate::verdict::Severity;

/// Default allowed commit types per the Conventional Commits spec and common extensions.
pub const DEFAULT_TYPES: &[&str] = &[
    "feat", "fix", "docs", "style", "refactor", "perf", "test", "build", "ci", "chore", "revert",
];

/// Check whether a commit message conforms to the Conventional Commits pattern:
/// `<type>[optional scope]: <description>`
///
/// Requirements:
/// - The type must be one of `allowed_types`.
/// - An optional scope in parentheses may follow the type.
/// - A colon followed by a space must separate the prefix from the description.
/// - The description must be non-empty.
pub fn is_conventional_commit(message: &str, allowed_types: &[&str]) -> bool {
    // Take only the first line (subject line).
    let subject = message.lines().next().unwrap_or("");

    // Find the colon-space separator.
    let Some(colon_pos) = subject.find(": ") else {
        return false;
    };

    let prefix = &subject[..colon_pos];
    let description = &subject[colon_pos + 2..];

    // Description must be non-empty.
    if description.trim().is_empty() {
        return false;
    }

    // Parse type and optional scope from prefix.
    // Valid forms: "feat", "feat(core)", "feat!"", "feat(core)!"
    let (ty, rest) = match prefix.find('(') {
        Some(paren_pos) => {
            let ty = &prefix[..paren_pos];
            let rest = &prefix[paren_pos..];
            // Scope must close with ')' optionally followed by '!'.
            if let Some(close) = rest.find(')') {
                let after_close = &rest[close + 1..];
                if after_close.is_empty() || after_close == "!" {
                    (ty, "")
                } else {
                    return false;
                }
            } else {
                return false;
            }
        }
        None => {
            // No scope. Allow optional trailing '!'.
            let ty = prefix.strip_suffix('!').unwrap_or(prefix);
            (ty, "")
        }
    };

    let _ = rest; // consumed during scope validation above

    allowed_types.iter().any(|&t| t == ty)
}

/// Check whether a commit message is a merge commit.
///
/// Matches GitHub's `Merge pull request #...` and git's `Merge branch '...'` patterns.
pub fn is_merge_commit(message: &str) -> bool {
    let subject = message.lines().next().unwrap_or("");
    subject.starts_with("Merge pull request #") || subject.starts_with("Merge branch ")
}

/// Classify the compliance of a set of commit messages.
///
/// Merge commits are excluded from the count.
/// - Non-compliant > 50% of non-merge commits: Error
/// - Non-compliant >= 1: Warning
/// - All compliant: Pass
pub fn classify_commit_compliance(messages: &[&str], allowed_types: &[&str]) -> Severity {
    let non_merge: Vec<&&str> = messages.iter().filter(|m| !is_merge_commit(m)).collect();

    if non_merge.is_empty() {
        return Severity::Pass;
    }

    let non_compliant_count = non_merge
        .iter()
        .filter(|m| !is_conventional_commit(m, allowed_types))
        .count();

    if non_compliant_count == 0 {
        Severity::Pass
    } else if non_compliant_count * 2 > non_merge.len() {
        Severity::Error
    } else {
        Severity::Warning
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- is_conventional_commit ---

    #[test]
    fn valid_feat() {
        assert!(is_conventional_commit("feat: add new rule", DEFAULT_TYPES));
    }

    #[test]
    fn valid_fix_with_scope() {
        assert!(is_conventional_commit(
            "fix(core): resolve panic",
            DEFAULT_TYPES
        ));
    }

    #[test]
    fn valid_breaking_change_bang() {
        assert!(is_conventional_commit(
            "feat!: breaking change",
            DEFAULT_TYPES
        ));
    }

    #[test]
    fn valid_scope_with_bang() {
        assert!(is_conventional_commit(
            "feat(api)!: breaking change",
            DEFAULT_TYPES
        ));
    }

    #[test]
    fn not_conventional_plain_message() {
        assert!(!is_conventional_commit("Update README", DEFAULT_TYPES));
    }

    #[test]
    fn no_space_after_colon() {
        assert!(!is_conventional_commit("feat:no space", DEFAULT_TYPES));
    }

    #[test]
    fn empty_description() {
        assert!(!is_conventional_commit("feat: ", DEFAULT_TYPES));
    }

    #[test]
    fn unknown_type() {
        assert!(!is_conventional_commit(
            "yolo: something",
            DEFAULT_TYPES
        ));
    }

    #[test]
    fn multiline_only_checks_subject() {
        assert!(is_conventional_commit(
            "feat: add feature\n\nBody text here",
            DEFAULT_TYPES
        ));
    }

    // --- is_merge_commit ---

    #[test]
    fn merge_pr() {
        assert!(is_merge_commit("Merge pull request #42 from user/branch"));
    }

    #[test]
    fn merge_branch() {
        assert!(is_merge_commit("Merge branch 'main' into feature"));
    }

    #[test]
    fn not_merge() {
        assert!(!is_merge_commit("feat: not a merge"));
    }

    // --- classify_commit_compliance ---

    #[test]
    fn all_compliant_returns_pass() {
        let msgs = vec!["feat: add feature", "fix: resolve bug", "docs: update readme"];
        assert_eq!(
            classify_commit_compliance(&msgs, DEFAULT_TYPES),
            Severity::Pass
        );
    }

    #[test]
    fn minority_non_compliant_returns_warning() {
        // 3 out of 10 non-compliant => Warning (30% <= 50%)
        let msgs = vec![
            "feat: a",
            "fix: b",
            "docs: c",
            "chore: d",
            "test: e",
            "ci: f",
            "refactor: g",
            "bad message 1",
            "bad message 2",
            "bad message 3",
        ];
        assert_eq!(
            classify_commit_compliance(&msgs, DEFAULT_TYPES),
            Severity::Warning
        );
    }

    #[test]
    fn majority_non_compliant_returns_error() {
        // 6 out of 10 non-compliant => Error (60% > 50%)
        let msgs = vec![
            "feat: a",
            "fix: b",
            "docs: c",
            "refactor: d",
            "bad 1",
            "bad 2",
            "bad 3",
            "bad 4",
            "bad 5",
            "bad 6",
        ];
        assert_eq!(
            classify_commit_compliance(&msgs, DEFAULT_TYPES),
            Severity::Error
        );
    }

    #[test]
    fn merge_commits_excluded_from_count() {
        // All non-merge commits are compliant; merge commits don't count.
        let msgs = vec![
            "feat: a",
            "Merge pull request #1 from user/branch",
            "Merge branch 'main'",
        ];
        assert_eq!(
            classify_commit_compliance(&msgs, DEFAULT_TYPES),
            Severity::Pass
        );
    }

    #[test]
    fn only_merge_commits_returns_pass() {
        let msgs = vec![
            "Merge pull request #1 from user/branch",
            "Merge branch 'main'",
        ];
        assert_eq!(
            classify_commit_compliance(&msgs, DEFAULT_TYPES),
            Severity::Pass
        );
    }

    #[test]
    fn single_non_compliant_returns_warning() {
        let msgs = vec!["feat: a", "bad message"];
        assert_eq!(
            classify_commit_compliance(&msgs, DEFAULT_TYPES),
            Severity::Warning
        );
    }

    #[test]
    fn exactly_half_non_compliant_returns_warning() {
        // 5 out of 10 => 50%, not > 50%, so Warning
        let msgs = vec![
            "feat: a",
            "fix: b",
            "docs: c",
            "chore: d",
            "test: e",
            "bad 1",
            "bad 2",
            "bad 3",
            "bad 4",
            "bad 5",
        ];
        assert_eq!(
            classify_commit_compliance(&msgs, DEFAULT_TYPES),
            Severity::Warning
        );
    }
}
