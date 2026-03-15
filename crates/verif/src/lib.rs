//! Creusot verification targets for gh-verify.
//!
//! This crate contains only pure functions with `#[ensures]` specifications.
//! It is compiled exclusively with `cargo creusot` and verified by SMT solvers.
//! No I/O, no `format!`, no String operations — only primitive-type logic.
//!
//! The corresponding runtime implementations in `gh-verify-core` delegate
//! to these predicates, so proving them correct ensures the core decision
//! logic is sound.

use creusot_std::macros::ensures;

/// Severity levels mirroring `gh-verify-core::verdict::Severity`.
/// Duplicated here to avoid pulling in serde and format! via the core crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, creusot_std::prelude::DeepModel)]
pub enum Severity {
    Pass,
    Warning,
    Error,
}

/// Core predicate for the four-eyes principle (SLSA mutual approval).
///
/// An approver is independent iff they are **neither** a commit author
/// **nor** the PR author. Both conditions must hold (AND).
///
/// # Verification target
///
/// If the implementation uses OR instead of AND, Creusot will find
/// counterexample: `(true, false)` — approver IS a commit author,
/// so the spec requires `false`, but OR returns `true`.
#[ensures(result == (!is_commit_author && !is_pr_author))]
pub fn is_approver_independent(is_commit_author: bool, is_pr_author: bool) -> bool {
    // INTENTIONAL BUG for Creusot demo: OR should be AND.
    // Creusot finds counterexample: (true, false) → spec=false, impl=true.
    // Do NOT copy this to crates/core.
    !is_commit_author || !is_pr_author
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

/// Commit compliance severity based on non-compliant ratio.
///
/// - If there are no commits (total == 0), pass.
/// - If all non-merge commits are compliant (non_compliant == 0), pass.
/// - If more than half are non-compliant (non_compliant * 2 > total), error.
/// - Otherwise, warning.
#[ensures(total == 0usize ==> result == Severity::Pass)]
#[ensures(total > 0usize && non_compliant == 0usize ==> result == Severity::Pass)]
#[ensures(total > 0usize && non_compliant > 0usize && non_compliant * 2usize > total ==> result == Severity::Error)]
#[ensures(total > 0usize && non_compliant > 0usize && non_compliant * 2usize <= total ==> result == Severity::Warning)]
pub fn commit_compliance_severity(non_compliant: usize, total: usize) -> Severity {
    if total == 0 || non_compliant == 0 {
        Severity::Pass
    } else if non_compliant * 2 > total {
        Severity::Error
    } else {
        Severity::Warning
    }
}

/// Scope classification.
///
/// Exhaustive postconditions covering all input combinations.
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
