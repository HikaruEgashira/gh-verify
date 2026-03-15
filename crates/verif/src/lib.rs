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

/// Issue linkage predicate.
///
/// Returns true iff there is at least one issue reference.
/// Mirrors `gh-verify-core::linkage::has_issue_linkage`.
#[ensures(result == (ref_count > 0usize))]
pub fn has_linkage(ref_count: usize) -> bool {
    ref_count > 0
}

/// Branch protection: protected branch membership check.
///
/// Models `is_protected_branch` as a boolean predicate. Since Creusot
/// cannot handle `&str` / `String`, we abstract the membership test to
/// a pre-computed boolean `is_member` that the caller resolves.
///
/// The postcondition is trivially `result == is_member`, ensuring the
/// runtime implementation faithfully forwards the membership answer.
#[ensures(result == is_member)]
pub fn is_protected_branch(is_member: bool) -> bool {
    is_member
}

/// Branch protection: admin merge detection.
///
/// A PR merged with zero reviews indicates an admin bypass of branch
/// protection rules. Returns `true` when `review_count == 0`.
#[ensures(review_count == 0usize ==> result == true)]
#[ensures(review_count > 0usize ==> result == false)]
pub fn is_admin_merge(review_count: usize) -> bool {
    review_count == 0
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

/// PR size classification.
///
/// Returns `Error` when either dimension exceeds its error threshold,
/// `Warning` when either exceeds its warning threshold, `Pass` otherwise.
/// Uses strict greater-than: exactly at the threshold does NOT trigger.
#[ensures(total_lines > error_lines || total_files > error_files ==> result == Severity::Error)]
#[ensures(!(total_lines > error_lines || total_files > error_files)
    && (total_lines > warn_lines || total_files > warn_files) ==> result == Severity::Warning)]
#[ensures(!(total_lines > error_lines || total_files > error_files)
    && !(total_lines > warn_lines || total_files > warn_files) ==> result == Severity::Pass)]
pub fn classify_pr_size(
    total_lines: usize,
    total_files: usize,
    warn_lines: usize,
    warn_files: usize,
    error_lines: usize,
    error_files: usize,
) -> Severity {
    if total_lines > error_lines || total_files > error_files {
        Severity::Error
    } else if total_lines > warn_lines || total_files > warn_files {
        Severity::Warning
    } else {
        Severity::Pass
    }
}

/// Test coverage: a source is covered when at least one matching test is found.
///
/// Mirrors `gh-verify-core::test_coverage::is_covered`.
#[ensures(matching_test_count > 0usize ==> result == true)]
#[ensures(matching_test_count == 0usize ==> result == false)]
pub fn is_covered(matching_test_count: usize) -> bool {
    matching_test_count > 0
}
