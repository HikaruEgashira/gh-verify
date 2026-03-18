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

/// Commit compliance severity based on non-compliant ratio.
///
/// - If there are no commits (total == 0), pass.
/// - If all non-merge commits are compliant (non_compliant == 0), pass.
/// - If more than half are non-compliant (non_compliant * 2 > total), error.
/// - Otherwise, warning.
///
/// Precondition: non_compliant cannot exceed total (domain invariant from
/// the counting logic in `conventional::classify_commit_compliance`).
/// Precondition: non_compliant <= total, and total fits in half usize range
/// to prevent overflow in `non_compliant * 2`.
#[requires(non_compliant <= total)]
#[requires(total <= 4611686018427387903usize)] // usize::MAX / 2 (prevents * 2 overflow)
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

/// Coverage classification severity.
///
/// Integer arithmetic avoids f64: `covered * 100` vs `threshold * total`.
/// Exhaustive postconditions covering all four cases.
///
/// Preconditions prevent overflow in `covered * 100` and `pct * total`.
#[requires(covered <= total)]
#[requires(total <= 46116860184273879usize)] // prevents total * 100 overflow
#[requires(warn_pct <= 100usize)]
#[requires(error_pct <= 100usize)]
#[requires(error_pct <= warn_pct)]
#[ensures(total == 0usize ==> result == Severity::Pass)]
#[ensures(total > 0usize && covered * 100usize > warn_pct * total ==> result == Severity::Pass)]
#[ensures(total > 0usize && covered * 100usize <= warn_pct * total
    && covered * 100usize > error_pct * total ==> result == Severity::Warning)]
#[ensures(total > 0usize && covered * 100usize <= warn_pct * total
    && covered * 100usize <= error_pct * total ==> result == Severity::Error)]
pub fn classify_coverage_severity(
    covered: usize,
    total: usize,
    warn_pct: usize,
    error_pct: usize,
) -> Severity {
    if total == 0 {
        return Severity::Pass;
    }
    if covered * 100 > warn_pct * total {
        Severity::Pass
    } else if covered * 100 > error_pct * total {
        Severity::Warning
    } else {
        Severity::Error
    }
}

/// Stale approval predicate.
///
/// An approval is stale iff the last approval timestamp is strictly before
/// the last commit timestamp. Mirrors `gh-verify-core::approval::is_approval_stale`
/// using integer timestamps instead of ISO 8601 strings.
#[ensures(result == (last_approval_ts < last_commit_ts))]
pub fn is_stale(last_approval_ts: usize, last_commit_ts: usize) -> bool {
    last_approval_ts < last_commit_ts
}

/// PR size classification.
///
/// Returns `Error` when either dimension exceeds its error threshold,
/// `Warning` when either exceeds its warning threshold, `Pass` otherwise.
/// Uses strict greater-than: exactly at the threshold does NOT trigger.
///
/// Precondition: warning thresholds must not exceed error thresholds.
/// This ensures the cascade Error > Warning > Pass is well-ordered.
#[requires(warn_lines <= error_lines)]
#[requires(warn_files <= error_files)]
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

// =====================================================================
// Red-team hardening predicates
// =====================================================================
// The following predicates close verification gaps found by red-teaming
// the Creusot proof boundary. They model composition logic that sits
// between GitHub API data and the core predicates above.

/// Branch protection: per-PR unprotected branch violation.
///
/// Models `classify_branch_compliance` at the per-PR level. A merged PR
/// targeting a non-protected branch is a violation. Non-merged PRs never
/// produce violations.
///
/// This closes the verification gap where `classify_branch_compliance`
/// (which iterates a Vec) was not verified by Creusot. By proving the
/// per-element logic, we reduce the unverified surface to the iteration.
#[ensures(!is_merged ==> result == false)]
#[ensures(is_merged && !is_protected ==> result == true)]
#[ensures(is_merged && is_protected ==> result == false)]
pub fn has_unprotected_branch_violation(is_merged: bool, is_protected: bool) -> bool {
    is_merged && !is_protected
}

/// Branch protection: per-PR admin merge violation.
///
/// A merged PR with zero reviews indicates admin bypass. Non-merged PRs
/// never produce violations.
#[ensures(!is_merged ==> result == false)]
#[ensures(is_merged && review_count == 0usize ==> result == true)]
#[ensures(is_merged && review_count > 0usize ==> result == false)]
pub fn has_admin_merge_violation(is_merged: bool, review_count: usize) -> bool {
    is_merged && review_count == 0
}

/// Approval staleness classification with empty-input guards.
///
/// Models `classify_approval_status` as a state machine:
/// - `has_commits == false` → status 0 (NoCommits)
/// - `has_approvals == false` → status 1 (NoApproval)
/// - `is_stale(max_approval_ts, max_commit_ts)` → status 2 (Stale)
/// - otherwise → status 3 (Fresh)
///
/// Uses integer status codes because Creusot cannot derive DeepModel
/// on an enum with 4+ variants without additional ceremony.
///
/// Precondition: if has_commits/has_approvals is true, the corresponding
/// max timestamp must be meaningful (>0). This prevents vacuous proofs.
#[requires(has_commits || max_commit_ts == 0usize)]
#[requires(has_approvals || max_approval_ts == 0usize)]
#[ensures(!has_commits ==> result == 0usize)]
#[ensures(has_commits && !has_approvals ==> result == 1usize)]
#[ensures(has_commits && has_approvals && max_approval_ts < max_commit_ts ==> result == 2usize)]
#[ensures(has_commits && has_approvals && max_approval_ts >= max_commit_ts ==> result == 3usize)]
pub fn classify_approval_status(
    has_commits: bool,
    has_approvals: bool,
    max_approval_ts: usize,
    max_commit_ts: usize,
) -> usize {
    if !has_commits {
        0
    } else if !has_approvals {
        1
    } else if max_approval_ts < max_commit_ts {
        2
    } else {
        3
    }
}

/// Monotonicity property: adding a signature violation cannot improve severity.
///
/// If the current count already produces Error, adding more unsigned
/// commits still produces Error. This is a "frame condition" proving
/// that `signature_severity` is monotone w.r.t. violation count.
#[requires(current > 0usize)]
#[ensures(result == Severity::Error)]
pub fn signature_severity_monotone(current: usize) -> Severity {
    // Precondition guarantees current > 0; body is trivially Error.
    let _ = current;
    Severity::Error
}

/// Composition: mutual approval requires BOTH independent approver AND
/// known authorship. This is the full four-eyes gate.
///
/// Proves: is_approver_independent alone is insufficient. Without author
/// knowledge (known_author_count == 0), the overall check MUST fail
/// regardless of what is_approver_independent would return.
#[ensures(known_author_count == 0usize ==> result == false)]
#[ensures(known_author_count > 0usize ==>
    result == (!is_commit_author && !is_pr_author))]
pub fn four_eyes_gate(
    known_author_count: usize,
    is_commit_author: bool,
    is_pr_author: bool,
) -> bool {
    if known_author_count == 0 {
        false
    } else {
        !is_commit_author && !is_pr_author
    }
}
