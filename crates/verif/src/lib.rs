//! Creusot verification targets for gh-verify.
//!
//! This crate contains only pure functions with `#[ensures]` specifications.
//! It is compiled exclusively with `cargo creusot` and verified by SMT solvers.
//! No I/O, no `format!`, no String operations — only primitive-type logic.
//!
//! The corresponding runtime implementations in `gh-verify-core` delegate
//! to these predicates, so proving them correct ensures the core decision
//! logic is sound.

use creusot_std::macros::{ensures, requires};

/// Severity levels mirroring `gh-verify-core::verdict::Severity`.
/// Duplicated here to avoid pulling in serde and format! via the core crate.
#[derive(Debug, Clone, Copy, creusot_std::prelude::DeepModel)]
pub enum Severity {
    Pass,
    Warning,
    Error,
}

/// Core predicate for the four-eyes principle (SLSA mutual approval).
///
/// An approver is independent iff they are **neither** a commit author
/// **nor** the PR author. Both conditions must hold (AND).
#[ensures(result == (!is_commit_author && !is_pr_author))]
pub fn is_approver_independent(is_commit_author: bool, is_pr_author: bool) -> bool {
    !is_commit_author && !is_pr_author
}

/// Core predicate for PR coverage check.
///
/// A commit is uncovered iff it is **not** a merge commit **and** has
/// no associated PR.
#[ensures(result == (!is_merge && !has_pr))]
pub fn is_uncovered_commit(is_merge: bool, has_pr: bool) -> bool {
    !is_merge && !has_pr
}

/// Signature check severity.
///
/// Pass iff zero unsigned commits; Error otherwise.
#[ensures(unsigned_count == 0usize ==> result == Severity::Pass)]
#[ensures(unsigned_count > 0usize ==> result == Severity::Error)]
pub fn signature_severity(unsigned_count: usize) -> Severity {
    if unsigned_count == 0 {
        Severity::Pass
    } else {
        Severity::Error
    }
}

/// PR coverage severity.
///
/// Error iff there are uncovered commits; Pass otherwise.
#[ensures(uncovered_count == 0usize ==> result == Severity::Pass)]
#[ensures(uncovered_count > 0usize ==> result == Severity::Error)]
pub fn pr_coverage_severity(uncovered_count: usize) -> Severity {
    if uncovered_count == 0 {
        Severity::Pass
    } else {
        Severity::Error
    }
}

/// Scope classification.
///
/// Exhaustive postconditions covering all input combinations.
///
/// Precondition: union-find produces at most as many connected components
/// as there are code files (each file starts as its own component).
#[requires(components <= code_files_count)]
#[ensures(code_files_count <= 1usize ==> result == Severity::Pass)]
#[ensures(code_files_count > 1usize && components <= 1usize ==> result == Severity::Pass)]
#[ensures(code_files_count > 1usize && components == 2usize ==> result == Severity::Warning)]
#[ensures(code_files_count > 1usize && components >= 3usize ==> result == Severity::Error)]
pub fn classify_scope(code_files_count: usize, components: usize) -> Severity {
    if code_files_count <= 1 {
        return Severity::Pass;
    }
    match components {
        0 | 1 => Severity::Pass,
        2 => Severity::Warning,
        _ => Severity::Error,
    }
}

/// Build provenance severity.
///
/// Pass iff at least one attestation exists and all are verified.
/// Error if any attestation is unverified. Pass if no attestations
/// (NotApplicable handled at control level).
#[ensures(attestation_count == 0usize ==> result == Severity::Pass)]
#[ensures(attestation_count > 0usize && all_verified ==> result == Severity::Pass)]
#[ensures(attestation_count > 0usize && !all_verified ==> result == Severity::Error)]
pub fn build_provenance_severity(attestation_count: usize, all_verified: bool) -> Severity {
    if attestation_count == 0 {
        Severity::Pass
    } else if all_verified {
        Severity::Pass
    } else {
        Severity::Error
    }
}

/// Branch protection severity.
///
/// Pass iff both stale review dismissal and admin enforcement are enabled.
#[ensures(dismiss_stale && enforce_admins ==> result == Severity::Pass)]
#[ensures(!dismiss_stale || !enforce_admins ==> result == Severity::Error)]
pub fn branch_protection_severity(dismiss_stale: bool, enforce_admins: bool) -> Severity {
    if dismiss_stale && enforce_admins {
        Severity::Pass
    } else {
        Severity::Error
    }
}

/// Required reviewers severity.
///
/// Pass iff at least one reviewer is required.
#[ensures(required_reviews >= 1u32 ==> result == Severity::Pass)]
#[ensures(required_reviews == 0u32 ==> result == Severity::Error)]
pub fn required_reviewers_severity(required_reviews: u32) -> Severity {
    if required_reviews >= 1 {
        Severity::Pass
    } else {
        Severity::Error
    }
}
