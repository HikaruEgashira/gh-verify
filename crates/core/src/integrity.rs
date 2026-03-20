//! Release integrity verification logic.
//!
//! Pure functions that determine SLSA compliance from structured data.
//! These functions have no I/O and are amenable to formal verification
//! with Creusot.
//!
//! # SLSA Properties Verified
//!
//! 1. **Commit Signatures**: All commits in a release range must be cryptographically signed.
//! 2. **Mutual Approval**: No PR may be approved solely by its own author or commit author.
//! 3. **PR Coverage**: All non-merge commits must be associated with a pull request.

use crate::verdict::{RuleResult, Severity};

const RULE_ID: &str = "verify-release-integrity";

/// Truncate a SHA to 7 characters for display.
pub fn short_sha(sha: &str) -> &str {
    if sha.len() >= 7 { &sha[..7] } else { sha }
}

// --- Input data structures (I/O-free, owned) ---

/// A commit with its verification status.
#[derive(Debug, Clone)]
pub struct Commit {
    pub sha: String,
    pub message: String,
    pub verified: bool,
    pub author_login: Option<String>,
    /// Number of parent commits. Merge commits have 2+ parents.
    /// When unavailable (legacy callers), falls back to message heuristic.
    pub parent_count: Option<u8>,
}

impl Commit {
    pub fn short_sha(&self) -> &str {
        short_sha(&self.sha)
    }

    /// A commit is a merge iff it has 2+ parents.
    /// Falls back to message prefix only when parent_count is unavailable.
    pub fn is_merge(&self) -> bool {
        match self.parent_count {
            Some(count) => count >= 2,
            None => self.message.starts_with("Merge "),
        }
    }
}

/// A PR with its associated review information.
#[derive(Debug, Clone)]
pub struct PrWithReviews {
    pub pr_number: u32,
    pub pr_author: String,
    pub commit_authors: Vec<String>,
    pub approvers: Vec<String>,
}

/// A commit's association with PRs.
#[derive(Debug, Clone)]
pub struct CommitPrAssoc {
    pub commit_sha: String,
    pub pr_numbers: Vec<u32>,
    pub is_merge: bool,
}

// --- Creusot-verifiable core predicates ---
//
// These pure functions operate on primitive types only, making them
// directly verifiable by Creusot's SMT backend. Complex-type functions
// delegate to these predicates, ensuring the critical logic is proven correct.
//
// If the AND in is_approver_independent were changed to OR, Creusot
// would produce a counterexample: (true, false) → spec says false, OR says true.

/// Core predicate for the four-eyes principle.
/// An approver is independent iff they are neither a commit author nor the PR author.
///
/// Core predicate for the four-eyes principle.
/// Verified by Creusot in `gh-verify-verif` crate.
pub fn is_approver_independent(is_commit_author: bool, is_pr_author: bool) -> bool {
    !is_commit_author && !is_pr_author
}

/// Core predicate for PR coverage.
/// Verified by Creusot in `gh-verify-verif` crate.
pub fn is_uncovered_commit(is_merge: bool, has_pr: bool) -> bool {
    !is_merge && !has_pr
}

/// Core predicate for signature verification result severity.
/// Verified by Creusot in `gh-verify-verif` crate.
pub fn signature_severity(unsigned_count: usize) -> Severity {
    if unsigned_count == 0 {
        Severity::Pass
    } else {
        Severity::Error
    }
}

/// Core predicate for PR coverage severity.
/// Verified by Creusot in `gh-verify-verif` crate.
pub fn pr_coverage_severity(uncovered_count: usize) -> Severity {
    if uncovered_count == 0 {
        Severity::Pass
    } else {
        Severity::Error
    }
}

// --- Pure verification functions ---

/// Check that all commits are cryptographically signed.
///
/// The result is `Pass` if and only if every commit has `verified == true`.
pub fn check_commit_signatures(commits: &[Commit]) -> RuleResult {
    let unsigned: Vec<&Commit> = commits.iter().filter(|c| !c.verified).collect();
    let severity = signature_severity(unsigned.len());

    if severity == Severity::Pass {
        return RuleResult::pass(RULE_ID, "All commits are signed");
    }

    let mut detail = String::from("Unsigned commits:\n");
    for c in &unsigned {
        detail.push_str(&format!("  {}\n", c.short_sha()));
    }
    detail.push_str("Enable commit signing: git config commit.gpgsign true");

    RuleResult {
        rule_id: RULE_ID.to_string(),
        severity: Severity::Error,
        message: format!(
            "{} of {} commits are unsigned",
            unsigned.len(),
            commits.len()
        ),
        affected_files: unsigned.iter().map(|c| c.short_sha().to_string()).collect(),
        suggestion: Some(detail),
    }
}

/// Check that every PR has at least one approver who is not a commit author
/// and not the PR author (mutual approval / four-eyes principle).
///
/// # Formal specification (Creusot)
///
/// ```text
/// #[ensures(result.severity == Severity::Pass
///     <==> forall(|i: usize| i < prs@.len()
///          ==> has_independent_approver(&prs@[i])))]
/// ```
pub fn check_mutual_approval(prs: &[PrWithReviews]) -> RuleResult {
    let mut violations: Vec<(u32, String, &str)> = Vec::new();

    for pr in prs {
        match has_independent_approver(pr) {
            Some(true) => {} // OK
            Some(false) => {
                violations.push((
                    pr.pr_number,
                    pr.pr_author.clone(),
                    "no independent approver",
                ));
            }
            None => {
                violations.push((
                    pr.pr_number,
                    pr.pr_author.clone(),
                    "commit authorship unknown — cannot verify four-eyes principle",
                ));
            }
        }
    }

    if violations.is_empty() {
        return RuleResult::pass(RULE_ID, "All PRs have independent approval");
    }

    let mut detail = String::new();
    for (number, author, reason) in &violations {
        detail.push_str(&format!("  PR #{number}: author={author}, {reason}\n"));
    }

    RuleResult {
        rule_id: RULE_ID.to_string(),
        severity: Severity::Error,
        message: format!(
            "{} PRs lack independent approval (commit author != approver)",
            violations.len()
        ),
        affected_files: violations
            .iter()
            .map(|(n, _, _)| format!("PR #{n}"))
            .collect(),
        suggestion: Some(detail),
    }
}

/// Determine whether a PR has at least one approver who is independent
/// (neither a commit author nor the PR author).
///
/// Returns `None` if `commit_authors` is empty, indicating the check
/// cannot be meaningfully performed (four-eyes principle requires
/// knowing commit authorship).
///
/// # Formal specification (Creusot)
///
/// ```text
/// #[ensures(result == true <==>
///     exists(|j: usize| j < pr.approvers@.len()
///         && !pr.commit_authors.contains(&pr.approvers[j])
///         && pr.approvers[j] != pr.pr_author))]
/// ```
fn has_independent_approver(pr: &PrWithReviews) -> Option<bool> {
    if pr.commit_authors.is_empty() {
        return None; // Cannot verify four-eyes without commit authorship
    }
    for approver in &pr.approvers {
        let is_commit_author = pr.commit_authors.iter().any(|a| a == approver);
        let is_pr_author = approver == &pr.pr_author;
        if is_approver_independent(is_commit_author, is_pr_author) {
            return Some(true);
        }
    }
    Some(false)
}

/// Check that all non-merge commits are associated with at least one PR.
///
/// # Formal specification (Creusot)
///
/// ```text
/// #[ensures(result.severity == Severity::Pass
///     <==> forall(|i: usize| i < assocs@.len()
///          ==> assocs@[i].is_merge || !assocs@[i].pr_numbers.is_empty()))]
/// ```
pub fn check_pr_coverage(assocs: &[CommitPrAssoc]) -> RuleResult {
    let uncovered: Vec<&CommitPrAssoc> = assocs
        .iter()
        .filter(|a| is_uncovered_commit(a.is_merge, !a.pr_numbers.is_empty()))
        .collect();

    let severity = pr_coverage_severity(uncovered.len());

    if severity == Severity::Pass {
        return RuleResult::pass(RULE_ID, "All commits are covered by PRs");
    }

    let short_shas: Vec<String> = uncovered
        .iter()
        .map(|a| short_sha(&a.commit_sha).to_string())
        .collect();

    RuleResult {
        rule_id: RULE_ID.to_string(),
        severity,
        message: format!(
            "{} commits have no associated PR (direct pushes)",
            uncovered.len()
        ),
        affected_files: short_shas,
        suggestion: Some(
            "All changes should go through pull requests for proper review.".to_string(),
        ),
    }
}

/// Run all release integrity checks and return aggregated results.
pub fn verify_release_integrity(
    commits: &[Commit],
    prs: &[PrWithReviews],
    assocs: &[CommitPrAssoc],
) -> Vec<RuleResult> {
    let mut results = Vec::new();

    let sig_result = check_commit_signatures(commits);
    let approval_result = check_mutual_approval(prs);
    let coverage_result = check_pr_coverage(assocs);

    // Only include non-pass results, or a single pass if all pass
    let all_pass = sig_result.severity == Severity::Pass
        && approval_result.severity == Severity::Pass
        && coverage_result.severity == Severity::Pass;

    if all_pass {
        results.push(RuleResult::pass(
            RULE_ID,
            "All release integrity checks passed",
        ));
    } else {
        if sig_result.severity != Severity::Pass {
            results.push(sig_result);
        }
        if approval_result.severity != Severity::Pass {
            results.push(approval_result);
        }
        if coverage_result.severity != Severity::Pass {
            results.push(coverage_result);
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_commit(sha: &str, verified: bool, author: &str) -> Commit {
        Commit {
            sha: sha.to_string(),
            message: "feat: something".to_string(),
            verified,
            author_login: Some(author.to_string()),
            parent_count: Some(1),
        }
    }

    fn make_merge_commit(sha: &str) -> Commit {
        Commit {
            sha: sha.to_string(),
            message: "Merge pull request #42".to_string(),
            verified: true,
            author_login: Some("bot".to_string()),
            parent_count: Some(2),
        }
    }

    // --- check_commit_signatures ---

    #[test]
    fn all_signed_returns_pass() {
        let commits = vec![
            make_commit("aaaaaaa1234567", true, "alice"),
            make_commit("bbbbbbb1234567", true, "bob"),
        ];
        let result = check_commit_signatures(&commits);
        assert_eq!(result.severity, Severity::Pass);
    }

    #[test]
    fn unsigned_commit_returns_error() {
        let commits = vec![
            make_commit("aaaaaaa1234567", true, "alice"),
            make_commit("bbbbbbb1234567", false, "bob"),
        ];
        let result = check_commit_signatures(&commits);
        assert_eq!(result.severity, Severity::Error);
        assert_eq!(result.affected_files, vec!["bbbbbbb"]);
    }

    #[test]
    fn all_unsigned_returns_error_with_count() {
        let commits = vec![
            make_commit("aaaaaaa1234567", false, "alice"),
            make_commit("bbbbbbb1234567", false, "bob"),
        ];
        let result = check_commit_signatures(&commits);
        assert_eq!(result.severity, Severity::Error);
        assert!(result.message.contains("2 of 2"));
    }

    // --- check_mutual_approval ---

    #[test]
    fn independent_approver_returns_pass() {
        let prs = vec![PrWithReviews {
            pr_number: 1,
            pr_author: "alice".to_string(),
            commit_authors: vec!["alice".to_string()],
            approvers: vec!["bob".to_string()],
        }];
        let result = check_mutual_approval(&prs);
        assert_eq!(result.severity, Severity::Pass);
    }

    #[test]
    fn self_approval_returns_error() {
        let prs = vec![PrWithReviews {
            pr_number: 1,
            pr_author: "alice".to_string(),
            commit_authors: vec!["alice".to_string()],
            approvers: vec!["alice".to_string()],
        }];
        let result = check_mutual_approval(&prs);
        assert_eq!(result.severity, Severity::Error);
    }

    /// ORロジックでは誤って true を返す反例: commit author が approver の場合
    #[test]
    fn commit_author_cannot_approve() {
        let prs = vec![PrWithReviews {
            pr_number: 1,
            pr_author: "alice".to_string(),
            commit_authors: vec!["bob".to_string()],
            approvers: vec!["bob".to_string()],
        }];
        let result = check_mutual_approval(&prs);
        assert_eq!(result.severity, Severity::Error);
    }

    /// ORロジックでは誤って true を返す反例: PR author が approver の場合
    #[test]
    fn pr_author_cannot_approve() {
        let prs = vec![PrWithReviews {
            pr_number: 1,
            pr_author: "alice".to_string(),
            commit_authors: vec!["bob".to_string()],
            approvers: vec!["alice".to_string()],
        }];
        let result = check_mutual_approval(&prs);
        assert_eq!(result.severity, Severity::Error);
    }

    #[test]
    fn no_approvers_returns_error() {
        let prs = vec![PrWithReviews {
            pr_number: 1,
            pr_author: "alice".to_string(),
            commit_authors: vec!["alice".to_string()],
            approvers: vec![],
        }];
        let result = check_mutual_approval(&prs);
        assert_eq!(result.severity, Severity::Error);
    }

    // --- check_pr_coverage ---

    #[test]
    fn all_covered_returns_pass() {
        let assocs = vec![
            CommitPrAssoc {
                commit_sha: "aaa".to_string(),
                pr_numbers: vec![1],
                is_merge: false,
            },
            CommitPrAssoc {
                commit_sha: "bbb".to_string(),
                pr_numbers: vec![2],
                is_merge: false,
            },
        ];
        let result = check_pr_coverage(&assocs);
        assert_eq!(result.severity, Severity::Pass);
    }

    #[test]
    fn merge_commit_without_pr_is_ok() {
        let assocs = vec![CommitPrAssoc {
            commit_sha: "aaa".to_string(),
            pr_numbers: vec![],
            is_merge: true,
        }];
        let result = check_pr_coverage(&assocs);
        assert_eq!(result.severity, Severity::Pass);
    }

    #[test]
    fn uncovered_non_merge_returns_error() {
        let assocs = vec![CommitPrAssoc {
            commit_sha: "aaaaaaa1234567".to_string(),
            pr_numbers: vec![],
            is_merge: false,
        }];
        let result = check_pr_coverage(&assocs);
        assert_eq!(result.severity, Severity::Error);
    }

    // --- verify_release_integrity (integration) ---

    #[test]
    fn all_pass_returns_single_pass() {
        let commits = vec![make_commit("aaa1234567", true, "alice")];
        let prs = vec![PrWithReviews {
            pr_number: 1,
            pr_author: "alice".to_string(),
            commit_authors: vec!["alice".to_string()],
            approvers: vec!["bob".to_string()],
        }];
        let assocs = vec![CommitPrAssoc {
            commit_sha: "aaa1234567".to_string(),
            pr_numbers: vec![1],
            is_merge: false,
        }];
        let results = verify_release_integrity(&commits, &prs, &assocs);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Pass);
    }

    #[test]
    fn mixed_failures_returns_multiple_results() {
        let commits = vec![make_commit("aaa1234567", false, "alice")];
        let prs = vec![PrWithReviews {
            pr_number: 1,
            pr_author: "alice".to_string(),
            commit_authors: vec!["alice".to_string()],
            approvers: vec!["alice".to_string()],
        }];
        let assocs = vec![CommitPrAssoc {
            commit_sha: "aaa1234567".to_string(),
            pr_numbers: vec![],
            is_merge: false,
        }];
        let results = verify_release_integrity(&commits, &prs, &assocs);
        assert!(results.len() >= 2);
        assert!(results.iter().any(|r| r.severity == Severity::Error));
    }

    // --- Specification property tests ---

    /// Property: check_commit_signatures returns Pass iff all commits are verified.
    /// This directly tests the biconditional in the formal spec.
    #[test]
    fn signature_biconditional() {
        // Forward: all verified => Pass
        let all_verified = vec![make_commit("aaa", true, "a"), make_commit("bbb", true, "b")];
        assert_eq!(
            check_commit_signatures(&all_verified).severity,
            Severity::Pass
        );

        // Backward (contrapositive): not all verified => not Pass
        let not_all_verified = vec![
            make_commit("aaa", true, "a"),
            make_commit("bbb", false, "b"),
        ];
        assert_ne!(
            check_commit_signatures(&not_all_verified).severity,
            Severity::Pass
        );
    }

    /// Property: check_pr_coverage returns Pass iff all non-merge commits have PRs.
    #[test]
    fn coverage_biconditional() {
        // Forward: all covered => Pass
        let covered = vec![CommitPrAssoc {
            commit_sha: "aaa".to_string(),
            pr_numbers: vec![1],
            is_merge: false,
        }];
        assert_eq!(check_pr_coverage(&covered).severity, Severity::Pass);

        // Backward: not covered => not Pass
        let uncovered = vec![CommitPrAssoc {
            commit_sha: "aaa".to_string(),
            pr_numbers: vec![],
            is_merge: false,
        }];
        assert_ne!(check_pr_coverage(&uncovered).severity, Severity::Pass);
    }

    /// Property: merge commits are always excluded from coverage check.
    #[test]
    fn merge_commits_excluded_from_coverage() {
        let merge_only = vec![make_merge_commit("aaa"), make_merge_commit("bbb")];
        let assocs: Vec<CommitPrAssoc> = merge_only
            .iter()
            .map(|c| CommitPrAssoc {
                commit_sha: c.sha.clone(),
                pr_numbers: vec![],
                is_merge: c.is_merge(),
            })
            .collect();
        assert_eq!(check_pr_coverage(&assocs).severity, Severity::Pass);
    }

    // --- verif↔core equivalence tests (exhaustive for boolean predicates) ---

    #[test]
    fn is_approver_independent_exhaustive_equivalence() {
        // Exhaustive truth table: both inputs are bool, so 4 combinations
        for &ca in &[false, true] {
            for &pa in &[false, true] {
                let result = is_approver_independent(ca, pa);
                let spec = !ca && !pa;
                assert_eq!(
                    result, spec,
                    "is_approver_independent({ca}, {pa}): got {result}, spec {spec}"
                );
            }
        }
    }

    #[test]
    fn is_uncovered_commit_exhaustive_equivalence() {
        for &merge in &[false, true] {
            for &has_pr in &[false, true] {
                let result = is_uncovered_commit(merge, has_pr);
                let spec = !merge && !has_pr;
                assert_eq!(
                    result, spec,
                    "is_uncovered_commit({merge}, {has_pr}): got {result}, spec {spec}"
                );
            }
        }
    }

    #[test]
    fn signature_severity_equivalence() {
        assert_eq!(signature_severity(0), Severity::Pass);
        for count in 1..=100 {
            assert_eq!(
                signature_severity(count),
                Severity::Error,
                "signature_severity({count}) should be Error"
            );
        }
    }

    #[test]
    fn pr_coverage_severity_equivalence() {
        assert_eq!(pr_coverage_severity(0), Severity::Pass);
        for count in 1..=100 {
            assert_eq!(
                pr_coverage_severity(count),
                Severity::Error,
                "pr_coverage_severity({count}) should be Error"
            );
        }
    }

    /// Exhaustive equivalence for classify_scope against Creusot spec.
    #[test]
    fn classify_scope_exhaustive_equivalence() {
        use crate::scope::classify_scope;
        for files in 0..=20usize {
            for comps in 0..=20usize {
                let result = classify_scope(files, comps);
                let spec = if files <= 1 {
                    Severity::Pass
                } else if comps <= 1 {
                    Severity::Pass
                } else if comps == 2 {
                    Severity::Warning
                } else {
                    Severity::Error
                };
                assert_eq!(
                    result, spec,
                    "classify_scope({files}, {comps}): got {result:?}, spec {spec:?}"
                );
            }
        }
    }

    // --- is_merge hardening tests ---

    #[test]
    fn is_merge_uses_parent_count_over_message() {
        // Spoofed message but parent_count=1 → NOT a merge
        let spoofed = Commit {
            sha: "aaa".to_string(),
            message: "Merge evil direct push".to_string(),
            verified: true,
            author_login: Some("attacker".to_string()),
            parent_count: Some(1),
        };
        assert!(!spoofed.is_merge(), "parent_count=1 must override message");

        // Real merge with parent_count=2
        let real_merge = Commit {
            sha: "bbb".to_string(),
            message: "feat: not a merge message".to_string(),
            verified: true,
            author_login: Some("bot".to_string()),
            parent_count: Some(2),
        };
        assert!(real_merge.is_merge(), "parent_count=2 is a merge");
    }

    #[test]
    fn is_merge_falls_back_to_message_when_parent_count_unavailable() {
        let legacy = Commit {
            sha: "ccc".to_string(),
            message: "Merge pull request #99".to_string(),
            verified: true,
            author_login: None,
            parent_count: None,
        };
        assert!(legacy.is_merge(), "fallback to message prefix");

        let not_merge = Commit {
            sha: "ddd".to_string(),
            message: "feat: add feature".to_string(),
            verified: true,
            author_login: None,
            parent_count: None,
        };
        assert!(!not_merge.is_merge());
    }

    // --- has_independent_approver edge cases ---

    #[test]
    fn empty_commit_authors_returns_error() {
        let prs = vec![PrWithReviews {
            pr_number: 1,
            pr_author: "alice".to_string(),
            commit_authors: vec![], // unknown authorship
            approvers: vec!["bob".to_string()],
        }];
        let result = check_mutual_approval(&prs);
        assert_eq!(
            result.severity,
            Severity::Error,
            "empty commit_authors must fail — four-eyes unverifiable"
        );
    }

}

#[cfg(test)]
#[path = "tests/integrity_hardening.rs"]
mod integrity_hardening;
