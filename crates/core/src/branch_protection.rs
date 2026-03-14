//! Branch protection compliance verification logic.
//!
//! Pure functions that determine whether PRs in a release were merged
//! into protected branches with proper review.
//!
//! # Properties Verified
//!
//! 1. **Protected Branch**: All merged PRs target a protected branch.
//! 2. **Admin Merge**: No PR was merged without at least one review (admin bypass).

use crate::verdict::{RuleResult, Severity};

const RULE_ID: &str = "verify-branch-protection";

/// Default protected branches when none are configured.
const DEFAULT_PROTECTED: &[&str] = &["main", "master"];

// --- Input data structures (I/O-free, owned) ---

/// PR metadata relevant to branch protection checks.
#[derive(Debug, Clone)]
pub struct PrBranchInfo {
    pub number: u32,
    pub base_ref: String,
    pub review_count: u32,
    pub merged: bool,
}

/// The kind of branch protection violation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViolationKind {
    /// PR was merged to a branch outside the protected set.
    UnprotectedBranch,
    /// PR was merged with zero reviews (admin bypass).
    AdminMerge,
}

/// A single branch protection violation.
#[derive(Debug, Clone)]
pub struct BranchViolation {
    pub pr_number: u32,
    pub base_ref: String,
    pub kind: ViolationKind,
}

// --- Core predicates ---

/// Returns true if `branch` is in the set of protected branches.
pub fn is_protected_branch(branch: &str, protected_branches: &[&str]) -> bool {
    protected_branches.iter().any(|&b| b == branch)
}

/// Classify each merged PR for branch protection compliance.
///
/// A merged PR targeting a non-protected branch produces an `UnprotectedBranch` violation.
/// A merged PR with zero reviews produces an `AdminMerge` violation.
/// Non-merged PRs are ignored.
pub fn classify_branch_compliance(
    prs: &[PrBranchInfo],
    protected_branches: &[&str],
) -> Vec<BranchViolation> {
    let branches = if protected_branches.is_empty() {
        DEFAULT_PROTECTED
    } else {
        protected_branches
    };

    let mut violations = Vec::new();
    for pr in prs {
        if !pr.merged {
            continue;
        }
        if !is_protected_branch(&pr.base_ref, branches) {
            violations.push(BranchViolation {
                pr_number: pr.number,
                base_ref: pr.base_ref.clone(),
                kind: ViolationKind::UnprotectedBranch,
            });
        }
        if pr.review_count == 0 {
            violations.push(BranchViolation {
                pr_number: pr.number,
                base_ref: pr.base_ref.clone(),
                kind: ViolationKind::AdminMerge,
            });
        }
    }
    violations
}

/// Run branch protection checks and return rule results.
pub fn check_branch_protection(
    prs: &[PrBranchInfo],
    protected_branches: &[&str],
) -> Vec<RuleResult> {
    let violations = classify_branch_compliance(prs, protected_branches);

    if violations.is_empty() {
        return vec![RuleResult::pass(
            RULE_ID,
            "All PRs target protected branches with proper review",
        )];
    }

    let admin_merges: Vec<&BranchViolation> = violations
        .iter()
        .filter(|v| v.kind == ViolationKind::AdminMerge)
        .collect();

    let unprotected: Vec<&BranchViolation> = violations
        .iter()
        .filter(|v| v.kind == ViolationKind::UnprotectedBranch)
        .collect();

    let mut results = Vec::new();

    if !admin_merges.is_empty() {
        let mut detail = String::from("PRs merged without review (admin bypass):\n");
        for v in &admin_merges {
            detail.push_str(&format!("  PR #{} -> {}\n", v.pr_number, v.base_ref));
        }

        results.push(RuleResult {
            rule_id: RULE_ID.to_string(),
            severity: Severity::Error,
            message: format!(
                "{} PRs merged without any review (possible admin bypass)",
                admin_merges.len()
            ),
            affected_files: admin_merges
                .iter()
                .map(|v| format!("PR #{}", v.pr_number))
                .collect(),
            suggestion: Some(detail),
        });
    }

    if !unprotected.is_empty() {
        let mut detail = String::from("PRs merged to unprotected branches:\n");
        for v in &unprotected {
            detail.push_str(&format!("  PR #{} -> {}\n", v.pr_number, v.base_ref));
        }

        results.push(RuleResult {
            rule_id: RULE_ID.to_string(),
            severity: Severity::Warning,
            message: format!(
                "{} PRs merged to unprotected branches",
                unprotected.len()
            ),
            affected_files: unprotected
                .iter()
                .map(|v| format!("PR #{}", v.pr_number))
                .collect(),
            suggestion: Some(detail),
        });
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pr(number: u32, base_ref: &str, review_count: u32, merged: bool) -> PrBranchInfo {
        PrBranchInfo {
            number,
            base_ref: base_ref.to_string(),
            review_count,
            merged,
        }
    }

    // --- is_protected_branch ---

    #[test]
    fn main_is_protected_by_default() {
        assert!(is_protected_branch("main", DEFAULT_PROTECTED));
        assert!(is_protected_branch("master", DEFAULT_PROTECTED));
    }

    #[test]
    fn feature_branch_not_protected() {
        assert!(!is_protected_branch("feature-x", DEFAULT_PROTECTED));
    }

    #[test]
    fn custom_protected_branches() {
        let branches = &["main", "develop"];
        assert!(is_protected_branch("develop", branches));
        assert!(!is_protected_branch("staging", branches));
    }

    // --- classify_branch_compliance ---

    #[test]
    fn all_prs_merged_to_main_pass() {
        let prs = vec![
            make_pr(1, "main", 2, true),
            make_pr(2, "main", 1, true),
        ];
        let violations = classify_branch_compliance(&prs, &["main"]);
        assert!(violations.is_empty());
    }

    #[test]
    fn pr_merged_to_feature_branch_violation() {
        let prs = vec![
            make_pr(1, "main", 1, true),
            make_pr(2, "feature-branch", 1, true),
        ];
        let violations = classify_branch_compliance(&prs, &["main"]);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].kind, ViolationKind::UnprotectedBranch);
        assert_eq!(violations[0].pr_number, 2);
    }

    #[test]
    fn custom_protected_develop_passes() {
        let prs = vec![make_pr(1, "develop", 1, true)];
        let violations = classify_branch_compliance(&prs, &["main", "develop"]);
        assert!(violations.is_empty());
    }

    #[test]
    fn admin_merge_zero_reviews() {
        let prs = vec![make_pr(1, "main", 0, true)];
        let violations = classify_branch_compliance(&prs, &["main"]);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].kind, ViolationKind::AdminMerge);
    }

    #[test]
    fn non_merged_pr_ignored() {
        let prs = vec![make_pr(1, "feature-branch", 0, false)];
        let violations = classify_branch_compliance(&prs, &["main"]);
        assert!(violations.is_empty());
    }

    #[test]
    fn no_prs_pass() {
        let violations = classify_branch_compliance(&[], &["main"]);
        assert!(violations.is_empty());
    }

    #[test]
    fn empty_protected_branches_uses_defaults() {
        let prs = vec![make_pr(1, "main", 1, true)];
        let violations = classify_branch_compliance(&prs, &[]);
        assert!(violations.is_empty());
    }

    // --- check_branch_protection (RuleResult level) ---

    #[test]
    fn all_pass_returns_single_pass() {
        let prs = vec![make_pr(1, "main", 1, true)];
        let results = check_branch_protection(&prs, &["main"]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Pass);
    }

    #[test]
    fn unprotected_branch_returns_warning() {
        let prs = vec![make_pr(1, "feature-branch", 1, true)];
        let results = check_branch_protection(&prs, &["main"]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Warning);
    }

    #[test]
    fn admin_merge_returns_error() {
        let prs = vec![make_pr(1, "main", 0, true)];
        let results = check_branch_protection(&prs, &["main"]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Error);
    }

    #[test]
    fn no_prs_returns_pass() {
        let results = check_branch_protection(&[], &["main"]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Pass);
    }

    #[test]
    fn both_violations_returns_error_and_warning() {
        let prs = vec![make_pr(1, "feature-branch", 0, true)];
        let results = check_branch_protection(&prs, &["main"]);
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|r| r.severity == Severity::Error));
        assert!(results.iter().any(|r| r.severity == Severity::Warning));
    }
}
