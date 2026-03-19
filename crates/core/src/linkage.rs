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
    "fixes", "fix", "fixed", "closes", "close", "closed", "resolves", "resolve", "resolved",
];

/// Common acronym prefixes that should NOT be treated as Jira project keys.
const JIRA_BLOCKLIST: &[&str] = &[
    "UTF", "HTTP", "RFC", "CVE", "ISO", "SHA", "SSL", "TLS", "TCP", "UDP", "DNS", "SSH", "API",
    "URL", "URI", "XML", "JSON", "YAML", "TOML", "HTML", "CSS", "ANSI", "ASCII", "IEEE", "IETF",
    "SMTP", "IMAP", "LDAP", "SAML", "CORS", "CSRF", "ECDSA", "HMAC",
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
///
/// All indexing operates on `Vec<char>` to avoid byte/char index confusion
/// with non-ASCII input. The keyword text for the output is reconstructed
/// from `body_chars` (original casing) rather than slicing `body` by byte.
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
            if i + kw_chars.len() < chars.len() && chars[i..i + kw_chars.len()] == kw_chars[..] {
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
                if j < chars.len()
                    && chars[j] == '#'
                    && let Some((num_str, end)) = parse_digits(&body_chars, j + 1)
                {
                    // Reconstruct keyword from original chars (preserves casing)
                    let kw_original: String = body_chars[i..i + kw_chars.len()].iter().collect();
                    let full = format!("{kw_original} #{num_str}");
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

        if matched_keyword {
            continue;
        }

        // Bare #N (not preceded by alphanumeric or &)
        if chars[i] == '#' {
            let preceded_ok = i == 0 || (!chars[i - 1].is_alphanumeric() && chars[i - 1] != '&');
            if preceded_ok && let Some((num_str, end)) = parse_digits(&body_chars, i + 1) {
                refs.push(IssueReference {
                    kind: IssueRefKind::GitHubIssue,
                    value: format!("#{num_str}"),
                });
                i = end;
                continue;
            }
        }

        i += 1;
    }
}

/// Parse a run of ASCII digits starting at `start`, returning the digit string and end index.
///
/// Returns `None` if there are no digits **or** if the digit run is immediately
/// followed by an alphanumeric character, `_`, or `-`. This prevents matching
/// `#123abc` as a GitHub issue reference while still accepting `#123`, `#123 `,
/// `#123.`, and `#123!`.
fn parse_digits(chars: &[char], start: usize) -> Option<(String, usize)> {
    let mut end = start;
    while end < chars.len() && chars[end].is_ascii_digit() {
        end += 1;
    }
    if end == start {
        return None;
    }
    // Reject if digits are followed by word-like characters (e.g. #123abc)
    if end < chars.len() {
        let next = chars[end];
        if next.is_alphanumeric() || next == '_' || next == '-' {
            return None;
        }
    }
    let s: String = chars[start..end].iter().collect();
    Some((s, end))
}

/// Extract Jira-style ticket references: `[A-Z]{2,}-\d+`.
///
/// Rejects prefixes in [`JIRA_BLOCKLIST`] (common acronyms like UTF, HTTP, etc.).
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

        let prefix: String = chars[alpha_start..alpha_start + alpha_len].iter().collect();

        // Reject well-known acronyms
        if JIRA_BLOCKLIST.iter().any(|b| *b == prefix) {
            i = j;
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
///
/// Handles both whitespace-delimited URLs and Markdown link syntax
/// `[text](url)`.
fn extract_urls(body: &str, refs: &mut Vec<IssueReference>) {
    // Strategy 1: scan for `https://` or `http://` anywhere in the text
    // and extract to the next whitespace or closing delimiter.
    let mut search_start = 0;
    while search_start < body.len() {
        // Find next URL-like prefix
        let rest = &body[search_start..];
        let offset = rest.find("https://").or_else(|| rest.find("http://"));

        let Some(pos) = offset else { break };
        let url_start = search_start + pos;

        // Determine end of URL: stop at whitespace, ')', '>', ']', or end of string
        let url_end = body[url_start..]
            .find(|c: char| c.is_whitespace() || c == ')' || c == '>' || c == ']')
            .map(|e| url_start + e)
            .unwrap_or(body.len());

        let url = body[url_start..url_end].trim_end_matches(['.', ',']);

        if url.contains("/issues/") || url.contains("/browse/") {
            refs.push(IssueReference {
                kind: IssueRefKind::Url,
                value: url.to_string(),
            });
        }

        search_start = url_end;
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
        assert_eq!(refs[0].value, "Closes #789");
    }

    #[test]
    fn github_issue_resolves_keyword() {
        let refs = extract_issue_references("resolves #012", &[]);
        assert!(has_issue_linkage(&refs));
        assert_eq!(refs[0].value, "resolves #012");
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
        let refs = extract_issue_references("https://github.com/owner/repo/issues/1", &[]);
        assert!(has_issue_linkage(&refs));
        assert_eq!(refs[0].kind, IssueRefKind::Url);
    }

    #[test]
    fn url_jira_browse() {
        let refs = extract_issue_references("See https://jira.example.com/browse/PROJ-123", &[]);
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

    // --- P1: Non-ASCII safety ---

    #[test]
    fn non_ascii_body_with_issue_ref() {
        // Multi-byte chars before issue reference must not panic
        let refs = extract_issue_references("あいう fixes #12", &[]);
        assert!(has_issue_linkage(&refs));
        assert_eq!(refs[0].value, "fixes #12");
    }

    #[test]
    fn non_ascii_body_bare_hash() {
        let refs = extract_issue_references("日本語テスト #99 です", &[]);
        assert!(has_issue_linkage(&refs));
        assert_eq!(refs[0].value, "#99");
    }

    #[test]
    fn emoji_body_with_issue_ref() {
        let refs = extract_issue_references("🎉🎊 closes #42", &[]);
        assert!(has_issue_linkage(&refs));
        assert_eq!(refs[0].value, "closes #42");
    }

    // --- P2: Markdown URL detection ---

    #[test]
    fn markdown_link_github_issues() {
        let body = "See [the issue](https://github.com/o/r/issues/1) for details";
        let refs = extract_issue_references(body, &[]);
        assert!(has_issue_linkage(&refs));
        assert_eq!(refs[0].kind, IssueRefKind::Url);
        assert!(refs[0].value.contains("/issues/1"));
    }

    #[test]
    fn markdown_link_jira_browse() {
        let body = "Related: [ticket](https://jira.example.com/browse/PROJ-456)";
        let refs = extract_issue_references(body, &[]);
        assert!(
            refs.iter()
                .any(|r| r.kind == IssueRefKind::Url && r.value.contains("/browse/"))
        );
    }

    // --- P3: Jira blocklist ---

    #[test]
    fn blocklist_utf8_not_jira() {
        let refs = extract_issue_references("Supports UTF-8 encoding", &[]);
        assert!(!refs.iter().any(|r| r.kind == IssueRefKind::JiraTicket));
    }

    #[test]
    fn blocklist_http_not_jira() {
        let refs = extract_issue_references("Returns HTTP-500 errors", &[]);
        assert!(!refs.iter().any(|r| r.kind == IssueRefKind::JiraTicket));
    }

    #[test]
    fn blocklist_rfc_not_jira() {
        let refs = extract_issue_references("Per RFC-9110 specification", &[]);
        assert!(!refs.iter().any(|r| r.kind == IssueRefKind::JiraTicket));
    }

    #[test]
    fn blocklist_cve_not_jira() {
        let refs = extract_issue_references("Fixes CVE-2024 vulnerability", &[]);
        assert!(!refs.iter().any(|r| r.kind == IssueRefKind::JiraTicket));
    }

    #[test]
    fn real_jira_ticket_still_works() {
        let refs = extract_issue_references("See PROJ-123 and MYAPP-456", &[]);
        assert_eq!(
            refs.iter()
                .filter(|r| r.kind == IssueRefKind::JiraTicket)
                .count(),
            2
        );
        assert!(refs.iter().any(|r| r.value == "PROJ-123"));
        assert!(refs.iter().any(|r| r.value == "MYAPP-456"));
    }

    // --- Trailing-character rejection (coderabbit fix) ---

    #[test]
    fn hash_followed_by_alpha_not_matched() {
        // #123abc is not a valid GitHub issue reference
        let refs = extract_issue_references("#123abc", &[]);
        assert!(!has_issue_linkage(&refs));
    }

    #[test]
    fn color_hex_not_matched() {
        // CSS hex color should not match
        let refs = extract_issue_references("color: #FF0000", &[]);
        assert!(!has_issue_linkage(&refs));
    }

    #[test]
    fn hash_followed_by_period_matched() {
        // Period is not alphanumeric, so #123. should match
        let refs = extract_issue_references("#123.", &[]);
        assert!(has_issue_linkage(&refs));
        assert_eq!(refs[0].value, "#123");
    }

    #[test]
    fn keyword_hash_followed_by_exclamation_matched() {
        // Exclamation is not alphanumeric, so fixes #123! should match
        let refs = extract_issue_references("fixes #123!", &[]);
        assert!(has_issue_linkage(&refs));
        assert_eq!(refs[0].value, "fixes #123");
    }

    // --- Biconditional property test ---

    /// Property: has_issue_linkage returns true iff extract_issue_references returns non-empty.
    #[test]
    fn linkage_biconditional() {
        // Forward: references exist => linkage
        let with_refs = extract_issue_references("fixes #1", &[]);
        assert!(has_issue_linkage(&with_refs));

        // Backward: no references => no linkage
        let without_refs = extract_issue_references("plain text", &[]);
        assert!(!has_issue_linkage(&without_refs));
    }

    // ================================================================
    // Mutation-hardening tests
    // ================================================================

    // --- parse_digits boundary mutations ---

    #[test]
    fn hash_followed_by_underscore_not_matched() {
        // Kills: removing '_' from rejection set in parse_digits
        let refs = extract_issue_references("#123_foo", &[]);
        assert!(!has_issue_linkage(&refs));
    }

    #[test]
    fn hash_followed_by_dash_not_matched() {
        // Kills: removing '-' from rejection set in parse_digits
        let refs = extract_issue_references("#123-foo", &[]);
        assert!(!has_issue_linkage(&refs));
    }

    #[test]
    fn hash_at_end_of_string_matched() {
        // Kills: requiring a trailing char after digits (end-of-string case)
        let refs = extract_issue_references("#999", &[]);
        assert!(has_issue_linkage(&refs));
        assert_eq!(refs[0].value, "#999");
    }

    #[test]
    fn hash_followed_by_space_matched() {
        let refs = extract_issue_references("#42 rest", &[]);
        assert!(has_issue_linkage(&refs));
        assert_eq!(refs[0].value, "#42");
    }

    #[test]
    fn hash_followed_by_newline_matched() {
        let refs = extract_issue_references("#42\nnext line", &[]);
        assert!(has_issue_linkage(&refs));
        assert_eq!(refs[0].value, "#42");
    }

    // --- Keyword variant coverage ---

    #[test]
    fn keyword_fix_singular() {
        let refs = extract_issue_references("fix #10", &[]);
        assert!(has_issue_linkage(&refs));
        assert_eq!(refs[0].value, "fix #10");
    }

    #[test]
    fn keyword_fixed_past_tense() {
        let refs = extract_issue_references("fixed #10", &[]);
        assert!(has_issue_linkage(&refs));
        assert_eq!(refs[0].value, "fixed #10");
    }

    #[test]
    fn keyword_close_singular() {
        let refs = extract_issue_references("close #10", &[]);
        assert!(has_issue_linkage(&refs));
        assert_eq!(refs[0].value, "close #10");
    }

    #[test]
    fn keyword_closed_past_tense() {
        let refs = extract_issue_references("closed #10", &[]);
        assert!(has_issue_linkage(&refs));
        assert_eq!(refs[0].value, "closed #10");
    }

    #[test]
    fn keyword_resolve_singular() {
        let refs = extract_issue_references("resolve #10", &[]);
        assert!(has_issue_linkage(&refs));
        assert_eq!(refs[0].value, "resolve #10");
    }

    #[test]
    fn keyword_resolved_past_tense() {
        let refs = extract_issue_references("resolved #10", &[]);
        assert!(has_issue_linkage(&refs));
        assert_eq!(refs[0].value, "resolved #10");
    }

    // --- Keyword word boundary ---

    #[test]
    fn keyword_must_be_at_word_boundary() {
        // "unfixes" should not match "fixes"
        let refs = extract_issue_references("unfixes #10", &[]);
        // Should match bare #10 but NOT as "fixes #10"
        assert!(has_issue_linkage(&refs));
        assert!(
            !refs.iter().any(|r| r.value.contains("fixes")),
            "should not extract 'fixes' from 'unfixes'"
        );
    }

    // --- Keyword with multiple spaces ---

    #[test]
    fn keyword_with_extra_spaces() {
        let refs = extract_issue_references("fixes   #10", &[]);
        assert!(has_issue_linkage(&refs));
        assert_eq!(refs[0].value, "fixes #10");
    }

    // --- Jira edge cases ---

    #[test]
    fn jira_exactly_two_letter_prefix_matches() {
        // Kills: alpha_len < 2 → alpha_len < 3
        let refs = extract_issue_references("AB-123 ticket", &[]);
        assert!(refs.iter().any(|r| r.kind == IssueRefKind::JiraTicket && r.value == "AB-123"));
    }

    #[test]
    fn jira_preceded_by_dash_not_matched() {
        // Kills: removing '-' from word boundary check at start
        let refs = extract_issue_references("X-PROJ-123", &[]);
        assert!(!refs.iter().any(|r| r.kind == IssueRefKind::JiraTicket));
    }

    #[test]
    fn jira_followed_by_dash_not_matched() {
        // Kills: removing '-' from word boundary check at end
        let refs = extract_issue_references("PROJ-123-extra", &[]);
        assert!(!refs.iter().any(|r| r.kind == IssueRefKind::JiraTicket));
    }

    #[test]
    fn jira_at_start_of_string() {
        let refs = extract_issue_references("PROJ-456", &[]);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].kind, IssueRefKind::JiraTicket);
        assert_eq!(refs[0].value, "PROJ-456");
    }

    #[test]
    fn jira_at_end_of_string() {
        let refs = extract_issue_references("see PROJ-456", &[]);
        assert!(refs.iter().any(|r| r.value == "PROJ-456"));
    }

    #[test]
    fn jira_no_digits_after_dash_not_matched() {
        let refs = extract_issue_references("PROJ-abc", &[]);
        assert!(!refs.iter().any(|r| r.kind == IssueRefKind::JiraTicket));
    }

    #[test]
    fn jira_preceded_by_alphanumeric_not_matched() {
        let refs = extract_issue_references("xPROJ-123", &[]);
        assert!(!refs.iter().any(|r| r.kind == IssueRefKind::JiraTicket));
    }

    // --- Jira blocklist: test all categories to prevent removal ---

    #[test]
    fn blocklist_ssl_not_jira() {
        let refs = extract_issue_references("Uses SSL-3 protocol", &[]);
        assert!(!refs.iter().any(|r| r.kind == IssueRefKind::JiraTicket));
    }

    #[test]
    fn blocklist_sha_not_jira() {
        let refs = extract_issue_references("SHA-256 hash", &[]);
        assert!(!refs.iter().any(|r| r.kind == IssueRefKind::JiraTicket));
    }

    #[test]
    fn blocklist_iso_not_jira() {
        let refs = extract_issue_references("ISO-8601 date format", &[]);
        assert!(!refs.iter().any(|r| r.kind == IssueRefKind::JiraTicket));
    }

    #[test]
    fn blocklist_tls_not_jira() {
        let refs = extract_issue_references("TLS-12 connection", &[]);
        assert!(!refs.iter().any(|r| r.kind == IssueRefKind::JiraTicket));
    }

    #[test]
    fn blocklist_api_not_jira() {
        let refs = extract_issue_references("API-42 endpoint", &[]);
        assert!(!refs.iter().any(|r| r.kind == IssueRefKind::JiraTicket));
    }

    // --- URL extraction mutations ---

    #[test]
    fn url_without_issues_or_browse_not_matched() {
        // Kills: removing the /issues/ and /browse/ filter
        let refs = extract_issue_references("https://github.com/owner/repo/pulls/1", &[]);
        assert!(!refs.iter().any(|r| r.kind == IssueRefKind::Url));
    }

    #[test]
    fn url_trailing_comma_stripped() {
        let refs =
            extract_issue_references("See https://github.com/o/r/issues/1, for details", &[]);
        assert!(has_issue_linkage(&refs));
        assert!(
            !refs[0].value.ends_with(','),
            "trailing comma should be stripped"
        );
    }

    #[test]
    fn url_trailing_period_stripped() {
        let refs =
            extract_issue_references("See https://github.com/o/r/issues/1. Next sentence.", &[]);
        assert!(has_issue_linkage(&refs));
        assert!(
            !refs[0].value.ends_with('.'),
            "trailing period should be stripped"
        );
    }

    #[test]
    fn url_http_also_matched() {
        // Kills: only matching https:// and not http://
        let refs = extract_issue_references("http://github.com/o/r/issues/1", &[]);
        assert!(has_issue_linkage(&refs));
        assert_eq!(refs[0].kind, IssueRefKind::Url);
    }

    #[test]
    fn url_in_angle_brackets() {
        let refs =
            extract_issue_references("Link: <https://github.com/o/r/issues/1> here", &[]);
        assert!(has_issue_linkage(&refs));
    }

    // --- Deduplication ---

    #[test]
    fn duplicate_references_deduplicated() {
        // Kills: removing dedup_by
        let refs = extract_issue_references("#42 and also #42", &[]);
        assert_eq!(
            refs.iter().filter(|r| r.value == "#42").count(),
            1,
            "duplicate #42 should be deduplicated"
        );
    }

    // --- Custom pattern edge cases ---

    #[test]
    fn custom_empty_pattern_no_match() {
        // Kills: removing early return on empty pattern
        let refs = extract_issue_references("anything here", &[""]);
        assert!(!refs.iter().any(|r| r.kind == IssueRefKind::Url && r.value.is_empty()));
    }

    #[test]
    fn custom_pattern_multiple_occurrences() {
        let refs = extract_issue_references("ABC-1 and ABC-1 again", &["ABC-1"]);
        // Custom matches both, but dedup may reduce
        assert!(refs.iter().any(|r| r.value == "ABC-1"));
    }

    #[test]
    fn custom_pattern_categorized_as_url_kind() {
        // Kills: changing the IssueRefKind for custom patterns
        // Use a pattern that won't also match as Jira/GitHub
        let refs = extract_issue_references("ticket: my-custom-ref-99", &["my-custom-ref-99"]);
        let custom_ref = refs.iter().find(|r| r.value == "my-custom-ref-99").unwrap();
        assert_eq!(custom_ref.kind, IssueRefKind::Url);
    }

    // --- has_issue_linkage exhaustive ---

    #[test]
    fn has_issue_linkage_empty_vec() {
        assert!(!has_issue_linkage(&[]));
    }

    #[test]
    fn has_issue_linkage_single_ref() {
        let refs = vec![IssueReference {
            kind: IssueRefKind::GitHubIssue,
            value: "#1".into(),
        }];
        assert!(has_issue_linkage(&refs));
    }

    // --- Jira URL dedup: Jira in URL should suppress standalone Jira match ---

    #[test]
    fn jira_in_url_suppresses_standalone() {
        let refs = extract_issue_references("https://jira.example.com/browse/PROJ-123", &[]);
        // Should have URL ref but NOT a separate Jira ticket ref for PROJ-123
        assert!(refs.iter().any(|r| r.kind == IssueRefKind::Url));
        assert!(
            !refs.iter().any(|r| r.kind == IssueRefKind::JiraTicket && r.value == "PROJ-123"),
            "PROJ-123 in URL should not also appear as standalone Jira ticket"
        );
    }

    // --- Bare # edge cases ---

    #[test]
    fn hash_at_start_of_string() {
        let refs = extract_issue_references("#1", &[]);
        assert!(has_issue_linkage(&refs));
        assert_eq!(refs[0].value, "#1");
    }

    #[test]
    fn hash_preceded_by_letter_not_matched() {
        // Kills: removing alphanumeric check for preceding char
        let refs = extract_issue_references("x#123", &[]);
        assert!(!has_issue_linkage(&refs));
    }

    #[test]
    fn hash_no_digits_not_matched() {
        let refs = extract_issue_references("# text", &[]);
        assert!(!has_issue_linkage(&refs));
    }
}
