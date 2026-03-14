use serde::{Deserialize, Serialize};

/// The kind of issue reference found in a PR body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IssueRefKind {
    GitHubIssue,
    JiraTicket,
    Url,
}

/// A single issue reference extracted from PR text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueReference {
    pub kind: IssueRefKind,
    pub value: String,
}

/// GitHub closing keyword prefixes (case-insensitive matching handled by caller).
const CLOSING_KEYWORDS: &[&str] = &[
    "fixes",
    "fix",
    "fixed",
    "closes",
    "close",
    "closed",
    "resolves",
    "resolve",
    "resolved",
];

/// Extract issue references from a PR body.
///
/// Recognized patterns:
/// - GitHub issue: `#123`, `fixes #456`, `closes #789`, `resolves #012`
/// - Jira format: `PROJ-123` (uppercase letters followed by dash and digits)
/// - URL format: URLs containing `/issues/` or `/browse/`
/// - Custom patterns provided by the caller
pub fn extract_issue_references(body: &str, custom_patterns: &[&str]) -> Vec<IssueReference> {
    let mut refs = Vec::new();

    // Extract URL references first (before other parsing mutates state)
    extract_urls(body, &mut refs);

    // Extract GitHub issue references (#N and keyword #N)
    extract_github_issues(body, &mut refs);

    // Extract Jira ticket references (PROJ-123)
    extract_jira_tickets(body, &mut refs);

    // Extract custom pattern matches
    for pattern in custom_patterns {
        extract_custom(body, pattern, &mut refs);
    }

    // Deduplicate by value
    refs.dedup_by(|a, b| a.value == b.value);
    refs
}

/// Returns true if the slice contains at least one issue reference.
pub fn has_issue_linkage(refs: &[IssueReference]) -> bool {
    !refs.is_empty()
}

/// Extract GitHub issue references: bare `#123` and keyword-prefixed `fixes #123`.
fn extract_github_issues(body: &str, refs: &mut Vec<IssueReference>) {
    let lower = body.to_lowercase();
    let chars: Vec<char> = lower.chars().collect();
    let body_chars: Vec<char> = body.chars().collect();

    let mut i = 0;
    while i < chars.len() {
        // Check for keyword + optional whitespace + #N
        let mut matched_keyword = false;
        for keyword in CLOSING_KEYWORDS {
            let kw_chars: Vec<char> = keyword.chars().collect();
            if i + kw_chars.len() < chars.len()
                && chars[i..i + kw_chars.len()] == kw_chars[..]
            {
                let after_kw = i + kw_chars.len();
                // Must be preceded by word boundary (start of string or non-alphanumeric)
                if i > 0 && chars[i - 1].is_alphanumeric() {
                    continue;
                }
                // Skip optional whitespace
                let mut j = after_kw;
                while j < chars.len() && chars[j] == ' ' {
                    j += 1;
                }
                if j < chars.len() && chars[j] == '#' {
                    if let Some((num_str, end)) = parse_digits(&body_chars, j + 1) {
                        let full = format!(
                            "{} #{}",
                            &body[i..i + kw_chars.len()],
                            num_str
                        );
                        refs.push(IssueReference {
                            kind: IssueRefKind::GitHubIssue,
                            value: full,
                        });
                        i = end;
                        matched_keyword = true;
                        break;
                    }
                }
            }
        }

        if matched_keyword {
            continue;
        }

        // Bare #N (not preceded by alphanumeric or &)
        if chars[i] == '#' {
            let preceded_ok = i == 0
                || (!chars[i - 1].is_alphanumeric() && chars[i - 1] != '&');
            if preceded_ok {
                if let Some((num_str, end)) = parse_digits(&body_chars, i + 1) {
                    refs.push(IssueReference {
                        kind: IssueRefKind::GitHubIssue,
                        value: format!("#{}", num_str),
                    });
                    i = end;
                    continue;
                }
            }
        }

        i += 1;
    }
}

/// Parse a run of ASCII digits starting at `start`, returning the digit string and end index.
fn parse_digits(chars: &[char], start: usize) -> Option<(String, usize)> {
    let mut end = start;
    while end < chars.len() && chars[end].is_ascii_digit() {
        end += 1;
    }
    if end > start {
        let s: String = chars[start..end].iter().collect();
        Some((s, end))
    } else {
        None
    }
}

/// Extract Jira-style ticket references: `[A-Z]{2,}-\d+`.
fn extract_jira_tickets(body: &str, refs: &mut Vec<IssueReference>) {
    let chars: Vec<char> = body.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Must start at word boundary
        if i > 0 && (chars[i - 1].is_alphanumeric() || chars[i - 1] == '-') {
            i += 1;
            continue;
        }

        // Scan uppercase letters (need at least 2)
        let alpha_start = i;
        let mut j = i;
        while j < chars.len() && chars[j].is_ascii_uppercase() {
            j += 1;
        }
        let alpha_len = j - alpha_start;
        if alpha_len < 2 {
            i += 1;
            continue;
        }

        // Must be followed by '-'
        if j >= chars.len() || chars[j] != '-' {
            i += 1;
            continue;
        }
        j += 1;

        // Must be followed by digits
        let digit_start = j;
        while j < chars.len() && chars[j].is_ascii_digit() {
            j += 1;
        }
        if j == digit_start {
            i += 1;
            continue;
        }

        // Must end at word boundary
        if j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '-') {
            i += 1;
            continue;
        }

        let ticket: String = chars[alpha_start..j].iter().collect();

        // Skip if this was already captured as part of a URL
        if !refs.iter().any(|r| r.value.contains(&ticket)) {
            refs.push(IssueReference {
                kind: IssueRefKind::JiraTicket,
                value: ticket,
            });
        }

        i = j;
    }
}

/// Extract URL references containing `/issues/` or `/browse/`.
fn extract_urls(body: &str, refs: &mut Vec<IssueReference>) {
    for word in body.split_whitespace() {
        // Also handle URLs wrapped in parentheses or angle brackets
        let word = word.trim_start_matches(['(', '<', '[']);
        let word = word.trim_end_matches([')', '>', ']', '.', ',']);

        if (word.starts_with("https://") || word.starts_with("http://"))
            && (word.contains("/issues/") || word.contains("/browse/"))
        {
            refs.push(IssueReference {
                kind: IssueRefKind::Url,
                value: word.to_string(),
            });
        }
    }
}

/// Extract matches for a custom literal pattern.
fn extract_custom(body: &str, pattern: &str, refs: &mut Vec<IssueReference>) {
    if pattern.is_empty() {
        return;
    }
    let mut start = 0;
    while let Some(pos) = body[start..].find(pattern) {
        let abs_pos = start + pos;
        let end = abs_pos + pattern.len();
        refs.push(IssueReference {
            kind: IssueRefKind::Url, // custom patterns categorized as Url
            value: body[abs_pos..end].to_string(),
        });
        start = end;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_issue_bare_hash() {
        let refs = extract_issue_references("Related to #123", &[]);
        assert!(has_issue_linkage(&refs));
        assert_eq!(refs[0].kind, IssueRefKind::GitHubIssue);
        assert_eq!(refs[0].value, "#123");
    }

    #[test]
    fn github_issue_fixes_keyword() {
        let refs = extract_issue_references("fixes #456", &[]);
        assert!(has_issue_linkage(&refs));
        assert_eq!(refs[0].value, "fixes #456");
    }

    #[test]
    fn github_issue_closes_keyword() {
        let refs = extract_issue_references("Closes #789", &[]);
        assert!(has_issue_linkage(&refs));
        assert!(refs[0].value.contains("#789"));
    }

    #[test]
    fn github_issue_resolves_keyword() {
        let refs = extract_issue_references("resolves #012", &[]);
        assert!(has_issue_linkage(&refs));
        assert!(refs[0].value.contains("#012"));
    }

    #[test]
    fn jira_ticket() {
        let refs = extract_issue_references("See PROJ-789 for details", &[]);
        assert!(has_issue_linkage(&refs));
        assert_eq!(refs[0].kind, IssueRefKind::JiraTicket);
        assert_eq!(refs[0].value, "PROJ-789");
    }

    #[test]
    fn url_github_issues() {
        let refs =
            extract_issue_references("https://github.com/owner/repo/issues/1", &[]);
        assert!(has_issue_linkage(&refs));
        assert_eq!(refs[0].kind, IssueRefKind::Url);
    }

    #[test]
    fn url_jira_browse() {
        let refs = extract_issue_references(
            "See https://jira.example.com/browse/PROJ-123",
            &[],
        );
        assert!(has_issue_linkage(&refs));
        assert_eq!(refs[0].kind, IssueRefKind::Url);
    }

    #[test]
    fn empty_body_no_linkage() {
        let refs = extract_issue_references("", &[]);
        assert!(!has_issue_linkage(&refs));
    }

    #[test]
    fn no_references_in_body() {
        let refs = extract_issue_references("Just a regular PR description.", &[]);
        assert!(!has_issue_linkage(&refs));
    }

    #[test]
    fn multiple_mixed_patterns() {
        let body = "fixes #123\nAlso related to PROJ-789 and https://github.com/o/r/issues/5";
        let refs = extract_issue_references(body, &[]);
        assert!(has_issue_linkage(&refs));
        assert!(refs.len() >= 3);
        let kinds: Vec<&IssueRefKind> = refs.iter().map(|r| &r.kind).collect();
        assert!(kinds.contains(&&IssueRefKind::GitHubIssue));
        assert!(kinds.contains(&&IssueRefKind::JiraTicket));
        assert!(kinds.contains(&&IssueRefKind::Url));
    }

    #[test]
    fn custom_pattern() {
        let refs = extract_issue_references("Ref: CUSTOM-42", &["CUSTOM-42"]);
        assert!(has_issue_linkage(&refs));
    }

    #[test]
    fn hash_in_html_entity_not_matched() {
        // &#123; should not match as a GitHub issue reference
        let refs = extract_issue_references("Use &#123; entity", &[]);
        assert!(!has_issue_linkage(&refs));
    }

    #[test]
    fn jira_single_letter_not_matched() {
        // Single letter prefix is not valid Jira
        let refs = extract_issue_references("X-123 should not match", &[]);
        assert!(!has_issue_linkage(&refs));
    }
}
